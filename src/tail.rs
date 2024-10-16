use aws_sdk_cloudformation::error::SdkError;
use aws_sdk_cloudformation::operation::describe_stack_events::DescribeStackEventsInput;
use aws_sdk_cloudformation::operation::describe_stacks::DescribeStacksInput;
use aws_sdk_cloudformation::types::StackEvent;
use aws_smithy_types_convert::date_time::DateTimeExt;
use chrono::{DateTime, Utc};
use eyre::{Context, Result};
use futures::future::join_all;
use notify_rust::Notification;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::time::Duration;
use term_table::{row::Row, Table, TableStyle};
use termcolor::{Color, ColorSpec, WriteColor};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::Instrument;

use crate::error::Error;
use crate::stacks::StackInfo;

fn event_sort_key(a: &StackEvent, b: &StackEvent) -> std::cmp::Ordering {
    let a_timestamp = a.timestamp.as_ref().unwrap().as_secs_f64();
    let b_timestamp = b.timestamp.as_ref().unwrap().as_secs_f64();

    a_timestamp.partial_cmp(&b_timestamp).unwrap()
}

#[cfg(target_os = "macos")]
fn notify(sound: impl AsRef<str>) -> Result<()> {
    let sound_name = sound.as_ref();
    Notification::new()
        .summary("Deploy finished")
        .body("deploy finished")
        .appname("cftail")
        .sound_name(sound_name)
        .show()?;
    Ok(())
}

// We can customise this further on linux, but I don't have a copy available
#[cfg(target_os = "linux")]
fn notify(_sound: impl AsRef<str>) -> Result<()> {
    Notification::new()
        .summary("Deploy finished")
        .body("deploy finished")
        .appname("cftail")
        .show()?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn notify(_sound: impl AsRef<str>) -> Result<()> {
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct TailConfig<'a> {
    pub(crate) since: DateTime<Utc>,
    pub(crate) stack_info: &'a StackInfo,
    pub(crate) show_separators: bool,
    pub(crate) show_notifications: bool,
    pub(crate) show_outputs: bool,
    pub(crate) show_resource_types: bool,
    pub(crate) sound: String,
}

#[derive(Debug, Clone, Copy)]
enum TailMode {
    None,
    Prefetch,
    Tail,
}

pub(crate) struct Tail<'a, W> {
    fetcher: Arc<dyn crate::aws::AwsCloudFormationClient + Sync + Send>,
    writer: &'a mut W,
    config: TailConfig<'a>,
    mode: TailMode,
    should_quit: Arc<AtomicBool>,
}

impl<'a, W> Tail<'a, W>
where
    W: WriteColor + Debug,
{
    pub(crate) fn new(
        config: TailConfig<'a>,
        fetcher: Arc<dyn crate::aws::AwsCloudFormationClient + Sync + Send>,
        writer: &'a mut W,
    ) -> Self {
        Self {
            config,
            fetcher,
            writer,
            mode: TailMode::None,
            should_quit: Arc::new(AtomicBool::new(false)),
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
            .await
            .wrap_err("fetching events for stack")?;
        tracing::debug!(nevents = all_events.len(), "got all past events");

        if all_events.is_empty() {
            tracing::debug!("no events found");
            return Ok(());
        }

        let mut latest_time = self.config.since;
        for e in &all_events {
            // let timestamp = crate::utils::parse_event_datetime(e.timestamp.as_str())?;
            let timestamp = e.timestamp().unwrap().to_chrono_utc().unwrap();
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
            match self.poll_step().await {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(e) => {
                    match e.downcast::<Error<()>>() {
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
            }

            tracing::trace!("sleeping");
            sleep(Duration::from_secs(5)).await;
        }
    }

    #[tracing::instrument(skip(self))]
    async fn poll_step(&mut self) -> Result<bool> {
        let all_events = self
            .fetch_events(self.config.stack_info.names.iter(), self.config.since)
            .await?;
        if all_events.is_empty() {
            tracing::debug!("no events found");
            return Ok(false);
        }

        let mut latest_time = self.config.since;
        for event in &all_events {
            let timestamp = event.timestamp().unwrap().to_chrono_utc().unwrap();
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

        if self.should_quit.load(atomic::Ordering::SeqCst) {
            return Ok(true);
        }

        Ok(false)
    }

    #[tracing::instrument(skip(self, event))]
    async fn print_event(&mut self, event: &StackEvent) -> Result<()> {
        let resource_name = event
            .logical_resource_id
            .as_ref()
            .expect("could not find logical_resource_id in response");
        let status = event
            .resource_status
            .as_ref()
            .expect("could not find resource_status in response");
        let stack_name = event.stack_name().unwrap();
        let timestamp = event.timestamp().unwrap().to_chrono_utc().unwrap();
        let status_reason = event.resource_status_reason.as_ref();
        let resource_type = event.resource_type.clone().unwrap_or("???".to_string());

        // timestamp
        write!(self.writer, "{timestamp}: ", timestamp = timestamp)
            .wrap_err("printing timestamp")?;

        // stack name and resource name, yellow if the resource name is the stack name, otherwise
        // in white
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

        if self.config.show_resource_types {
            // resource type
            write!(self.writer, " | ").wrap_err("writing separator")?;
            {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Magenta));
                self.writer.set_color(&spec).wrap_err("setting color")?;
                write!(self.writer, "{resource_type}").wrap_err("printing resource type")?;
                self.writer.reset().wrap_err("resetting color")?;
            }
        }

        write!(self.writer, " | ").wrap_err("writing separator")?;

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
            // if let TailMode::Tail = self.mode {
            if self.config.show_outputs {
                self.print_stack_outputs(event.stack_name().unwrap())
                    .await?;
            }
            // }
            if self.config.show_separators {
                self.print_separator().wrap_err("printing separator")?;
            }
            if self.config.show_notifications {
                if let TailMode::Tail = self.mode {
                    notify(&self.config.sound).wrap_err("showing notification")?;
                }
            }

            // signal to the main process that we should quit
            self.should_quit.store(true, atomic::Ordering::SeqCst);
        } else {
            writeln!(self.writer).wrap_err("printing end of event")?;
        }

        Ok(())
    }

    // get the list of stack outputs that have been deployed and print to the output
    #[tracing::instrument(skip(self))]
    async fn print_stack_outputs(&mut self, stack_name: &str) -> Result<()> {
        tracing::info!(%stack_name, "printing stack outputs");
        let input = DescribeStacksInput::builder()
            .stack_name(stack_name)
            .build()
            .wrap_err("building describe stacks input")?;
        let res = self.fetcher.describe_stacks(input).await?;
        let stacks = res.stacks();
        if stacks.len() != 1 {
            unreachable!(
                "unexpected number of stacks, found {}, expected 1",
                stacks.len()
            );
        }

        if let Some(outputs) = stacks[0].outputs.as_ref() {
            writeln!(self.writer, "\nOutputs:").unwrap();

            let mut table = Table::new();
            table.style = TableStyle::thin();
            table.add_row(Row::new(vec!["Name", "Value"]));

            for output in outputs {
                table.add_row(Row::new(vec![
                    output.output_key().unwrap(),
                    output.output_value().unwrap(),
                ]));
            }
            writeln!(self.writer, "{}", table.render()).unwrap();
        } else {
            tracing::debug!("no outputs found");
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
                let tx = tx.clone();
                let fetcher = Arc::clone(&self.fetcher);
                let stack_name = stack_name.clone();
                tracing::debug!("spawning task");
                tokio::spawn(async move {
                    tracing::debug!("spawned task");
                    let mut next_token: Option<String> = None;
                    let mut all_events = Vec::new();
                    let mut seen_event_ids = HashSet::new();

                    'poll: loop {
                        let input = DescribeStackEventsInput::builder()
                            .stack_name(stack_name.clone())
                            .set_next_token(next_token.clone())
                            .build()
                            .expect("constructing input for describe stack events");

                        tracing::debug!(input = ?input, "sending request with payload");
                        let res = fetcher
                            .describe_stack_events(input)
                            .instrument(tracing::debug_span!("fetching events"))
                            .await;

                        match res {
                            Ok(response) => {
                                tracing::debug!("got successful response");
                                for event in response.stack_events.unwrap() {
                                    let timestamp =
                                        event.timestamp().unwrap().to_chrono_utc().unwrap();

                                    // We know that the events are in reverse chronological
                                    // order, so if we witness an event with a timestamp
                                    // that's earlier than what we have already seen, then
                                    // we know that it has already been presented.
                                    if timestamp <= since {
                                        break 'poll;
                                    }

                                    // if we have seen the event already then skip the event
                                    if seen_event_ids.contains(&event.event_id) {
                                        continue;
                                    }

                                    seen_event_ids.insert(event.event_id.clone());

                                    all_events.push(event);
                                }

                                match response.next_token {
                                    Some(new_next_token) => next_token = Some(new_next_token),
                                    None => break,
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = ?e, "got failed response");
                                match e {
                                    SdkError::ServiceError(ref s) => {
                                        tracing::error!(error = ?s, "service error");
                                        return Err(Error::Client(e));
                                    }
                                    _ => {}
                                }
                            }
                        };
                    }

                    let _ = tx.send(all_events).await;

                    Ok::<(), _>(())
                })
            })
            .collect();

        for res in join_all(handles).await {
            let res = res?;
            if let Err(e) = res {
                // return Err(e);
                tracing::warn!(error = ?e, "error with task");
                eyre::bail!("error with task: {e:#}");
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

/*
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use termcolor::{ColorSpec, WriteColor};

    use crate::{
        stacks::StackInfo,
        tail::{Tail, TailConfig},
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

    struct MockClient;

    #[async_trait]
    impl crate::aws::AwsCloudFormationClient for MockClient {
        async fn describe_stacks(
            &self,
            _input: crate::aws::DescribeStacksInput,
        ) -> Result<crate::aws::DescribeStacksOutput, crate::aws::DescribeStacksError> {
            todo!()
        }

        async fn describe_stack_events(
            &self,
            input: crate::aws::DescribeStackEventsInput,
        ) -> Result<crate::aws::DescribeStackEventsOutput, crate::aws::DescribeStackEventsError>
        {
            Ok(crate::aws::DescribeStackEventsOutput {
                next_token: None,
                stack_events: vec![StackEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: "2020-11-17T10:38:57.149Z".to_string(),
                    logical_resource_id: Some("test-stack".to_string()),
                    resource_status: Some("UPDATE_COMPLETE".to_string()),
                    resource_type: Some("stack".to_string()),
                    stack_name: input.stack_name.unwrap(),
                    resource_status_reason: None,
                }],
            })
        }

        async fn describe_stack_resources(
            &self,
            _input: crate::aws::DescribeStackResourcesInput,
        ) -> Result<crate::aws::DescribeStackResourcesOutput, crate::aws::DescribeStackResourcesError>
        {
            todo!()
        }
    }

    #[tokio::test]
    async fn test_prefetch() {
        tracing_subscriber::fmt::init();
        use std::collections::HashSet;

        let client = MockClient {};
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
            show_separators: true,
            show_notifications: true,
            show_outputs: true,
            sound: "Ping".to_string(),
            show_resource_types: true,
        };
        let mut writer = StubWriter::default();

        let mut tail = Tail::new(config, Arc::new(client), &mut writer);

        tail.prefetch().await.unwrap();

        let buf = std::str::from_utf8(&writer.buf).unwrap();
        assert_eq!(
            buf,
            "2020-11-17T10:38:57.149Z: SampleStack - test-stack | stack | UPDATE_COMPLETE\n"
        );
    }
}
*/
