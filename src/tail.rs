use chrono::{DateTime, Utc};
use eyre::{Result, WrapErr};
use futures::future::join_all;
use notify_rust::Notification;
use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsInput, DescribeStacksInput, StackEvent,
};
use rusoto_core::RusotoError;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use termcolor::{Color, ColorSpec, WriteColor};
use tokio::sync::mpsc;
use tokio::time::delay_for;
use tracing::Instrument;

use crate::error::Error;
use crate::stacks::StackInfo;

fn event_sort_key(a: &StackEvent, b: &StackEvent) -> std::cmp::Ordering {
    let a_timestamp = DateTime::parse_from_rfc3339(&a.timestamp).unwrap();
    let b_timestamp = DateTime::parse_from_rfc3339(&b.timestamp).unwrap();

    a_timestamp.partial_cmp(&b_timestamp).unwrap()
}

#[cfg(target_os = "macos")]
fn notify() -> Result<()> {
    Notification::new()
        .summary("Deploy finished")
        .body("deploy finished")
        .appname("cftail")
        .sound_name("Ping")
        .show()?;
    Ok(())
}

// We can customise this further on linux, but I don't have a copy available
#[cfg(target_os = "linux")]
fn notify() -> Result<()> {
    Notification::new()
        .summary("Deploy finished")
        .body(&format!("deploy finished"))
        .appname("cftail")
        .show()?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn notify() -> Result<()> {
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct TailConfig<'a> {
    pub(crate) since: DateTime<Utc>,
    pub(crate) stack_info: &'a StackInfo,
    pub(crate) nested: bool,
    pub(crate) show_separators: bool,
    pub(crate) show_notifications: bool,
    pub(crate) show_outputs: bool,
}

#[derive(Debug, Clone, Copy)]
enum TailMode {
    None,
    Prefetch,
    Tail,
}

pub(crate) struct Tail<'a, W> {
    fetcher: Arc<CloudFormationClient>,
    writer: &'a mut W,
    config: TailConfig<'a>,
    mode: TailMode,
    prefetch_notified: bool,
}

impl<'a, W> Tail<'a, W>
where
    W: WriteColor + Debug,
{
    pub(crate) fn new(
        config: TailConfig<'a>,
        fetcher: Arc<CloudFormationClient>,
        writer: &'a mut W,
    ) -> Self {
        Self {
            config,
            fetcher,
            writer,
            mode: TailMode::None,
            prefetch_notified: false,
        }
    }

    // Fetch all of the events since the beginning of time, so that we can ensure all
    // of the events are sorted.
    #[tracing::instrument(skip(self))]
    pub(crate) async fn prefetch(&mut self) -> Result<()> {
        tracing::debug!("prefetching events");
        self.mode = TailMode::Prefetch;
        // fetch all of the stack events for the nested stacks
        let all_events = self
            .fetch_events(self.config.stack_info.names.iter(), self.config.since)
            .await?;
        tracing::debug!(nevents = all_events.len(), "got all past events");

        if all_events.is_empty() {
            tracing::debug!("no events found");
            return Ok(());
        }

        let mut latest_time = self.config.since;
        for e in &all_events {
            let timestamp = crate::utils::parse_event_datetime(e.timestamp.as_str())?;
            self.print_event(e).await.expect("printing");
            tracing::trace!(latest_time = ?latest_time, timestamp = ?timestamp, "later timestamp");
            if timestamp > latest_time {
                latest_time = timestamp;
            } else {
                tracing::warn!(latest_time = ?latest_time, timestamp = ?timestamp, "earlier timestamp");
            }
        }
        tracing::trace!(latest_time = ?latest_time, "setting config.since");
        self.config.since = latest_time;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn poll(&mut self) -> Result<()> {
        tracing::debug!(start_time = ?self.config.since, "showing logs from now");
        self.mode = TailMode::Tail;

        loop {
            if let Err(e) = self.poll_step().await {
                match e.downcast::<Error>() {
                    Ok(e) => match e {
                        Error::CredentialsExpired => {
                            // We have to surface this back up to the main
                            // function, as this will create a new client and
                            // try again
                            return Err(e).wrap_err("expired credentials");
                        }
                        _ => {
                            tracing::warn!(err = %e, "unhandled error");
                        }
                    },
                    Err(e) => tracing::error!(err = %e, "unhandled error"),
                }
            }

            tracing::trace!("sleeping");
            delay_for(Duration::from_secs(5)).await;
        }
    }

    #[tracing::instrument(skip(self))]
    async fn poll_step(&mut self) -> Result<()> {
        let all_events = self
            .fetch_events(self.config.stack_info.names.iter(), self.config.since)
            .await?;
        if all_events.is_empty() {
            tracing::debug!("no events found");
            return Ok(());
        }

        let mut latest_time = self.config.since;
        for event in &all_events {
            let timestamp = crate::utils::parse_event_datetime(event.timestamp.as_str())?;
            self.print_event(event).await.expect("printing");
            tracing::trace!(latest_time = ?latest_time, timestamp = ?timestamp, "later timestamp");
            if timestamp > latest_time {
                latest_time = timestamp;
            } else {
                tracing::warn!(latest_time = ?latest_time, timestamp = ?timestamp, "earlier timestamp");
            }
        }

        tracing::trace!(latest_time = ?latest_time, "setting config.since");
        self.config.since = latest_time;

        Ok(())
    }

    #[tracing::instrument(skip(self, event))]
    async fn print_event(&mut self, event: &rusoto_cloudformation::StackEvent) -> Result<()> {
        let resource_name = event
            .logical_resource_id
            .as_ref()
            .expect("could not find logical_resource_id in response");
        let status = event
            .resource_status
            .as_ref()
            .expect("could not find resource_status in response");
        let stack_name = event.stack_name.as_str();
        let timestamp = &event.timestamp;
        let status_reason = event.resource_status_reason.as_ref();

        write!(self.writer, "{timestamp}: ", timestamp = timestamp)
            .wrap_err("printing timestamp")?;
        if self
            .config
            .stack_info
            .original_names
            .contains(resource_name)
        {
            let mut spec = ColorSpec::new();
            spec.set_fg(Some(Color::Yellow));
            self.writer.set_color(&spec).wrap_err("setting color")?;
            write!(
                self.writer,
                "{stack_name} - {name}",
                stack_name = stack_name,
                name = resource_name
            )
            .wrap_err("printing resource name")?;
            self.writer.reset().wrap_err("resetting colour")?;
        } else {
            write!(
                self.writer,
                "{stack_name} - {name}",
                stack_name = stack_name,
                name = resource_name
            )
            .wrap_err("printing resource name")?;
        }

        write!(self.writer, " | ").wrap_err("printing pipe character")?;

        let stack_status = crate::stack_status::StackStatus::try_from(status.as_str())
            .expect("unhandled stack status");
        if let Some(spec) = stack_status.color_spec() {
            self.writer.set_color(&spec).wrap_err("setting color")?;
        }

        write!(self.writer, "{}", status).expect("printing status");
        self.writer.reset().wrap_err("resetting colour")?;

        if let Some(reason) = status_reason {
            writeln!(self.writer, " ({reason})", reason = reason)
                .wrap_err("printing failure reason")?;
        } else if stack_status.is_complete()
            && self
                .config
                .stack_info
                .original_names
                .contains(resource_name)
        {
            // the stack has finished deploying
            writeln!(self.writer, " 🎉✨🤘").wrap_err("printing finished line")?;
            if let TailMode::Tail = self.mode {
                if self.config.show_outputs {
                    self.print_stack_outputs(&event.stack_name).await?;
                }
            }
            if self.config.show_separators {
                self.print_separator().wrap_err("printing separator")?;
            }
            if self.config.show_notifications {
                if let TailMode::Tail = self.mode {
                    notify().wrap_err("showing notification")?;
                }
            }
        } else {
            writeln!(self.writer).wrap_err("printing end of event")?;
        }

        Ok(())
    }

    // get the list of stack outputs that have been deployed and print to the output
    #[tracing::instrument(skip(self))]
    async fn print_stack_outputs(&mut self, stack_name: &str) -> Result<()> {
        tracing::info!(%stack_name, "printing stack outputs");
        let input = DescribeStacksInput {
            stack_name: Some(stack_name.to_string()),
            ..Default::default()
        };
        let res = self.fetcher.describe_stacks(input).await.unwrap();
        match res.stacks {
            Some(stacks) => {
                if stacks.len() != 1 {
                    unreachable!(
                        "unexpected number of stacks, found {}, expected 1",
                        stacks.len()
                    );
                }

                if let Some(outputs) = stacks[0].outputs.as_ref() {
                    writeln!(self.writer, "\nOutputs:").unwrap();

                    let mut table = comfy_table::Table::new();
                    table.load_preset(comfy_table::presets::UTF8_FULL);
                    table.set_header(vec!["Name", "Value"]);

                    for output in outputs {
                        let name = output.output_key.as_ref().unwrap();
                        let value = output.output_value.as_ref().unwrap();
                        table.add_row(vec![name, value]);
                    }
                    writeln!(self.writer, "{}", table).unwrap();
                } else {
                    tracing::debug!("no outputs found");
                }
            }
            None => unreachable!(),
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn print_separator(&mut self) -> Result<()> {
        if let Some((w, _)) = term_size::dimensions() {
            let chars = vec!['-'; w];
            let sep: String = chars.iter().collect();
            writeln!(self.writer, "{}", sep).wrap_err("writing to writer")?;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, stacks))]
    async fn fetch_events(
        &mut self,
        stacks: impl Iterator<Item = &String>,
        since: DateTime<Utc>,
    ) -> Result<Vec<StackEvent>> {
        let (tx, mut rx) = mpsc::channel(self.config.stack_info.names.len());
        let handles: Vec<_> = stacks
            .map(|stack_name| {
                tracing::debug!(name = ?stack_name, "fetching events for stack");
                let mut tx = tx.clone();
                let fetcher = Arc::clone(&self.fetcher);
                let stack_name = stack_name.clone();
                tracing::debug!("spawning task");
                tokio::spawn(async move {
                    tracing::debug!("spawned task");
                    let mut next_token: Option<String> = None;
                    let mut all_events = Vec::new();

                    'poll: loop {
                        let input = DescribeStackEventsInput {
                            stack_name: Some(stack_name.clone()),
                            next_token: next_token.clone(),
                        };

                        tracing::debug!(input = ?input, "sending request with payload");
                        let res = fetcher
                            .describe_stack_events(input)
                            .instrument(tracing::debug_span!("fetching events"))
                            .await;

                        match res {
                            Ok(response) => {
                                tracing::debug!("got successful response");
                                match response.stack_events {
                                    Some(batch) => {
                                        for event in batch {
                                            let timestamp = crate::utils::parse_event_datetime(
                                                event.timestamp.as_str(),
                                            )?;

                                            // We know that the events are in reverse chronological
                                            // order, so if we witness an event with a timestamp
                                            // that's earlier than what we have already seen, then
                                            // we know that it has already been presented.
                                            if timestamp <= since {
                                                break 'poll;
                                            }
                                            all_events.push(event);
                                        }
                                    }
                                    None => {
                                        tracing::debug!("reached end of events");
                                        break;
                                    }
                                }

                                match response.next_token {
                                    Some(new_next_token) => next_token = Some(new_next_token),
                                    None => break,
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = ?e, "got failed response");
                                match e {
                                    RusotoError::Service(ref error) => {
                                        tracing::error!(err = %error, "rusoto error");
                                        return Err(Error::Rusoto(e)).wrap_err("rusoto error");
                                    }
                                    RusotoError::Credentials(ref creds) => {
                                        tracing::error!(creds = ?creds, "credentials err");
                                        return Err(Error::NoCredentials)
                                            .wrap_err("credentials error");
                                    }
                                    RusotoError::Unknown(response) => {
                                        let body_str = std::str::from_utf8(&response.body)
                                            .wrap_err(
                                                "error decoding response body as utf8 string",
                                            )?;
                                        let error = crate::error::ErrorResponse::from_str(body_str)
                                            .wrap_err("parsing error response")?;

                                        let underlying = match error.error.code.as_str() {
                                            "Throttling" => Error::RateLimitExceeded,
                                            "ExpiredToken" => Error::CredentialsExpired,
                                            "ValidationError" => Error::NoStack(stack_name.clone()),
                                            _ => Error::ErrorResponse(error),
                                        };
                                        return Err(underlying).wrap_err("rusoto error");
                                    }
                                    RusotoError::HttpDispatch(_e) => {
                                        // Do nothing, these are usually temporary
                                    }
                                    _ => {
                                        tracing::error!(err = ?e, "other sort of error");
                                        return Err(Error::Other(format!("{:?}", e)))
                                            .wrap_err("other error");
                                    }
                                }
                            }
                        };
                    }

                    tx.send(all_events)
                        .await
                        .wrap_err("error sending events over channel")?;

                    Ok::<(), eyre::Error>(())
                })
            })
            .collect();

        for res in join_all(handles).await {
            let res = res?;
            if let Err(e) = res {
                return Err(e);
            }
        }

        drop(tx);

        let mut all_events = Vec::new();
        tracing::debug!("waiting for events");
        while let Some(res) = rx.recv().await {
            all_events.extend(res);
        }

        all_events.sort_by(event_sort_key);

        Ok(all_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rusoto_mock::{
        MockCredentialsProvider, MockRequestDispatcher, MockResponseReader, ReadMockResponse,
    };

    #[derive(Debug, Default)]
    struct StubWriter {
        buf: Vec<u8>,
    }

    impl std::io::Write for StubWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl WriteColor for StubWriter {
        fn supports_color(&self) -> bool {
            true
        }

        fn set_color(&mut self, _spec: &ColorSpec) -> std::io::Result<()> {
            Ok(())
        }

        fn reset(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_prefetch() {
        tracing_subscriber::fmt::init();
        use std::collections::HashSet;

        let response = MockResponseReader::read_response("tests/responses", "single_event.xml");
        let dispatcher = MockRequestDispatcher::with_status(200).with_body(&response);
        let client =
            CloudFormationClient::new_with(dispatcher, MockCredentialsProvider, Default::default());
        let stacks = {
            let mut stacks = HashSet::new();
            stacks.insert(String::from("SampleStack"));
            stacks
        };
        let original_stack_names = {
            let mut s = HashSet::new();
            s.insert(String::from("SampleStack"));
            s
        };
        let stack_info = StackInfo {
            original_names: original_stack_names,
            names: stacks,
        };
        let config = TailConfig {
            since: Utc.timestamp(0, 0),
            stack_info: &stack_info,
            nested: false,
            show_separators: true,
            show_notifications: true,
            show_outputs: true,
        };
        let mut writer = StubWriter::default();

        let mut tail = Tail::new(config, Arc::new(client), &mut writer);

        tail.prefetch().await.unwrap();

        let buf = std::str::from_utf8(&writer.buf).unwrap();
        assert_eq!(
            buf,
            "2020-11-17T10:38:57.149Z: test-stack - test-stack | UPDATE_COMPLETE\n"
        );
    }
}
