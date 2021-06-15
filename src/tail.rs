use chrono::{DateTime, Utc};
use eyre::{Result, WrapErr};
use futures::future::join_all;
use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsInput, StackEvent,
};
use rusoto_core::RusotoError;
use std::collections::HashSet;
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

fn event_sort_key(a: &StackEvent, b: &StackEvent) -> std::cmp::Ordering {
    let a_timestamp = DateTime::parse_from_rfc3339(&a.timestamp).unwrap();
    let b_timestamp = DateTime::parse_from_rfc3339(&b.timestamp).unwrap();

    a_timestamp.partial_cmp(&b_timestamp).unwrap()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TailConfig<'a> {
    pub(crate) original_stack_name: &'a str,
    pub(crate) since: DateTime<Utc>,
    pub(crate) stacks: &'a HashSet<String>,
    pub(crate) nested: bool,
}

pub(crate) struct Tail<'a, W> {
    fetcher: Arc<CloudFormationClient>,
    writer: W,
    seen_events: &'a mut HashSet<String>,
    config: TailConfig<'a>,
}

impl<'a, W> Tail<'a, W>
where
    W: WriteColor + Debug,
{
    pub(crate) fn new(
        config: TailConfig<'a>,
        fetcher: Arc<CloudFormationClient>,
        writer: W,
        seen_events: &'a mut HashSet<String>,
    ) -> Self {
        Self {
            config,
            fetcher,
            writer,
            seen_events,
        }
    }

    // Fetch all of the events since the beginning of time, so that we can ensure all
    // of the events are sorted.
    // #[tracing::instrument(skip(self))]
    pub(crate) async fn prefetch(&mut self) -> Result<()> {
        tracing::debug!("prefetching events");
        // fetch all of the stack events for the nested stacks
        let all_events = self.fetch_events(self.config.stacks.iter()).await?;
        tracing::debug!(nevents = all_events.len(), "got all past events");

        if all_events.is_empty() {
            tracing::debug!("no events found");
            return Ok(());
        }

        for e in &all_events {
            let timestamp =
                DateTime::parse_from_rfc3339(e.timestamp.as_str()).expect("parsing timestamp");
            if timestamp > self.config.since {
                self.print_event(&e).expect("printing");
            }
            self.seen_events.insert(e.event_id.clone());
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn poll(&mut self) -> Result<()> {
        tracing::debug!(start_time = ?self.config.since, "showing logs from now");

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
        tracing::info!(n_seen_events = ?self.seen_events.len(), "running poll step");
        let all_events = self.fetch_events(self.config.stacks.iter()).await?;
        if all_events.is_empty() {
            tracing::debug!("no events found");
            return Ok(());
        }

        for event in &all_events {
            if self.seen_events.contains(&event.event_id) {
                continue;
            }

            self.print_event(&event).expect("printing");
            self.seen_events.insert(event.event_id.clone());
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, event))]
    fn print_event(&mut self, event: &rusoto_cloudformation::StackEvent) -> Result<()> {
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
        if resource_name == self.config.original_stack_name {
            let mut spec = ColorSpec::new();
            spec.set_fg(Some(Color::Yellow));
            self.writer.set_color(&spec).wrap_err("setting color")?;
            if self.config.nested {
                write!(
                    self.writer,
                    "{stack_name} - {name}",
                    stack_name = stack_name,
                    name = resource_name
                )
                .wrap_err("printing resource name")?;
            } else {
                write!(self.writer, "{name}", name = resource_name)
                    .wrap_err("printing resource name")?;
            }
            self.writer.reset().wrap_err("resetting colour")?;
        } else {
            if self.config.nested {
                write!(
                    self.writer,
                    "{stack_name} - {name}",
                    stack_name = stack_name,
                    name = resource_name
                )
                .wrap_err("printing resource name")?;
            } else {
                write!(self.writer, "{name}", name = resource_name)
                    .wrap_err("printing resource name")?;
            }
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
        } else {
            if stack_status.is_complete() && resource_name == self.config.original_stack_name {
                writeln!(self.writer, " 🎉✨🤘").wrap_err("printing finished line")?;
            } else {
                writeln!(self.writer, "").wrap_err("printing end of event")?;
            }
        }

        Ok(())
    }

    async fn fetch_events(
        &mut self,
        stacks: impl Iterator<Item = &String>,
    ) -> Result<Vec<StackEvent>> {
        let (tx, mut rx) = mpsc::channel(self.config.stacks.len());
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

                    loop {
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
                                        all_events.extend_from_slice(&batch);
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
                                tracing::debug!("got failed response");
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
                                            "ValidationError" => Error::NoStack,
                                            _ => Error::ErrorResponse(error),
                                        };
                                        return Err(underlying).wrap_err("rusoto error");
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

        join_all(handles).await;

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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use chrono::TimeZone;
//     use rusoto_mock::{
//         MockCredentialsProvider, MockRequestDispatcher, MockResponseReader, ReadMockResponse,
//     };

//     #[derive(Debug)]
//     struct StubWriter;

//     impl std::io::Write for StubWriter {
//         fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//             Ok(buf.len())
//         }

//         fn flush(&mut self) -> std::io::Result<()> {
//             Ok(())
//         }
//     }

//     impl WriteColor for StubWriter {
//         fn supports_color(&self) -> bool {
//             true
//         }

//         fn set_color(&mut self, _spec: &ColorSpec) -> std::io::Result<()> {
//             Ok(())
//         }

//         fn reset(&mut self) -> std::io::Result<()> {
//             Ok(())
//         }
//     }

//     #[tokio::test]
//     async fn test_prefetch() {
//         tracing_subscriber::fmt::init();

//         let response = MockResponseReader::read_response("tests/responses", "single_event.xml");
//         let dispatcher = MockRequestDispatcher::with_status(200).with_body(&response);
//         let client =
//             CloudFormationClient::new_with(dispatcher, MockCredentialsProvider, Default::default());
//         let mut seen_events = HashSet::new();
//         let mut tail = Tail::new(
//             &client,
//             StubWriter {},
//             "SampleStack",
//             Utc.timestamp(0, 0),
//             &mut seen_events,
//         );

//         tail.prefetch().await.unwrap();

//         assert_eq!(seen_events.len(), 1);
//     }
// }
