use crate::exponential_backoff::backoff;
use chrono::{DateTime, Utc};
use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsInput, StackEvent,
};
use rusoto_core::RusotoError;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::str::FromStr;
use std::time::Duration;
use termcolor::{Color, ColorSpec, WriteColor};
use tokio::time::delay_for;
use tracing::Instrument;

use crate::error::{AwsError, Error};

fn event_sort_key(a: &StackEvent, b: &StackEvent) -> std::cmp::Ordering {
    let a_timestamp = DateTime::parse_from_rfc3339(&a.timestamp).unwrap();
    let b_timestamp = DateTime::parse_from_rfc3339(&b.timestamp).unwrap();

    a_timestamp.partial_cmp(&b_timestamp).unwrap()
}

pub(crate) struct Tail<'a, W> {
    fetcher: &'a CloudFormationClient,
    writer: W,
    stack_name: &'a str,
    since: DateTime<Utc>,
    seen_events: &'a mut HashSet<String>,
    latest_event: Option<DateTime<Utc>>,
}

impl<'a, W> Tail<'a, W>
where
    W: WriteColor + Debug,
{
    pub(crate) fn new(
        fetcher: &'a CloudFormationClient,
        writer: W,
        stack_name: &'a str,
        since: DateTime<Utc>,
        seen_events: &'a mut HashSet<String>,
    ) -> Self {
        Self {
            fetcher,
            writer,
            stack_name,
            since,
            latest_event: None,
            seen_events,
        }
    }

    // Fetch all of the events since the beginning of time, so that we can ensure all
    // of the events are sorted.
    #[tracing::instrument(skip(self))]
    pub(crate) async fn prefetch(&mut self) -> Result<(), Error> {
        let mut all_events = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            tracing::debug!(next_token = ?next_token, "fetching more events");
            let input = DescribeStackEventsInput {
                stack_name: Some(self.stack_name.to_string()),
                next_token: next_token.clone(),
            };

            match self
                .fetcher
                .describe_stack_events(input)
                .instrument(tracing::debug_span!("fetching events"))
                .await
            {
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
                            return Err(Error::Rusoto(e));
                        }
                        RusotoError::Credentials(ref creds) => {
                            tracing::error!(creds = ?creds, "credentials err");
                            return Err(Error::Aws(crate::error::AwsError::NoCredentials));
                        }
                        _ => tracing::error!(err = ?e, "other sort of error"),
                    }
                }
            }
        }

        tracing::debug!(nevents = all_events.len(), "got all past events");

        all_events.sort_by(event_sort_key);

        if all_events.is_empty() {
            tracing::debug!("no events found");
            return Ok(());
        }

        let last_event = &all_events[all_events.len() - 1];
        self.latest_event = Some(
            DateTime::parse_from_rfc3339(&last_event.timestamp)
                .unwrap()
                .with_timezone(&Utc),
        );

        for e in &all_events {
            let timestamp =
                DateTime::parse_from_rfc3339(e.timestamp.as_str()).expect("parsing timestamp");
            if timestamp > self.since {
                self.print_event(&e).expect("printing");
            }
            self.seen_events.insert(e.event_id.clone());
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn poll(&mut self) -> Result<(), Error> {
        tracing::debug!(start_time = ?self.since, "showing logs from now");

        loop {
            if let Err(e) = self.poll_step().await {
                match e {
                    Error::Aws(AwsError::RateLimitExceeded) => {
                        tracing::warn!("rate limit exceeded");
                        delay_for(Duration::from_secs(10)).await;
                    }
                    // Surface the expired credentials up to the main outer loop so that a new
                    // client can be constructed.
                    Error::Aws(AwsError::CredentialExpired) => return Err(e),
                    _ => tracing::error!(err = %e, "unhandled error"),
                }
            }

            tracing::trace!("sleeping");
            delay_for(Duration::from_secs(5)).await;
        }
    }

    #[tracing::instrument(skip(self))]
    async fn poll_step(&mut self) -> Result<(), Error> {
        let input = DescribeStackEventsInput {
            stack_name: Some(self.stack_name.to_string()),
            ..Default::default()
        };

        let res = self
            .fetcher
            .describe_stack_events(input)
            .await
            .map_err(Error::Rusoto)?;

        let mut events = res.stack_events.unwrap_or_else(|| Vec::new());
        events.sort_by(event_sort_key);
        for event in events.into_iter() {
            let timestamp =
                DateTime::<Utc>::from_str(&event.timestamp).expect("parsing event time");
            // Filter on timestamp
            if timestamp < self.since {
                continue;
            }

            if self.seen_events.contains(&event.event_id) {
                continue;
            }

            self.print_event(&event).expect("printing");

            self.seen_events.insert(event.event_id);
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, event))]
    fn print_event(&mut self, event: &rusoto_cloudformation::StackEvent) -> Result<(), Error> {
        let resource_name = event.logical_resource_id.as_ref().unwrap();
        let status = event.resource_status.as_ref().unwrap();
        let timestamp = &event.timestamp;
        let status_reason = event.resource_status_reason.as_ref();

        write!(self.writer, "{timestamp}: ", timestamp = timestamp).map_err(|_| Error::Printing)?;
        if resource_name == self.stack_name {
            let mut spec = ColorSpec::new();
            spec.set_fg(Some(Color::Yellow));
            self.writer.set_color(&spec).unwrap();
            write!(self.writer, "{name}", name = resource_name).map_err(|_| Error::Printing)?;
            self.writer.reset().map_err(|_| Error::Printing)?;
        } else {
            write!(self.writer, "{name}", name = resource_name).map_err(|_| Error::Printing)?;
        }

        write!(self.writer, " | ").map_err(|_| Error::Printing)?;

        let stack_status = crate::stack_status::StackStatus::try_from(status.as_str())
            .expect("unhandled stack status");
        if let Some(spec) = stack_status.color_spec() {
            self.writer.set_color(&spec).map_err(|_| Error::Printing)?;
        }

        write!(self.writer, "{}", status).expect("printing");
        self.writer.reset().map_err(|_| Error::Printing)?;

        if let Some(reason) = status_reason {
            writeln!(self.writer, " ({reason})", reason = reason).map_err(|_| Error::Printing)?;
        } else {
            if stack_status.is_complete() && resource_name == self.stack_name {
                writeln!(self.writer, " 🎉✨🤘").map_err(|_| Error::Printing)?;
            } else {
                writeln!(self.writer, "").map_err(|_| Error::Printing)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusoto_mock::{MockCredentialsProvider, MockRequestDispatcher, MockResponseReader};

    #[tokio::test]
    async fn test_prefetch() {}

    #[tokio::test]
    async fn test_poll() {}
}
