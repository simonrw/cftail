use async_trait::async_trait;
use chrono::prelude::*;
use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsInput, StackEvent,
};
use rusoto_core::Region;
use std::collections::HashSet;
use std::fmt::Debug;
use std::time::Duration;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::time::delay_for;
use tracing::Instrument;

enum Error {
    CredentialTimeout,
    Other(Box<dyn std::error::Error>),
}

#[derive(StructOpt)]
struct Opts {
    stack_name: String,

    #[structopt(short, long)]
    since: Option<i64>,
}

#[async_trait]
trait Fetch {
    async fn fetch_events_since<S>(
        &self,
        stack_name: S,
        start_time: &DateTime<Utc>,
    ) -> Result<Vec<StackEvent>, Box<dyn std::error::Error>>
    where
        S: Into<String> + Send;

    async fn fetch_all_events<S>(
        &self,
        stack_name: S,
    ) -> Result<Vec<StackEvent>, Box<dyn std::error::Error>>
    where
        S: Into<String> + Send + Debug;
}

#[derive(Debug)]
struct Tail<'a, F, W> {
    fetcher: F,
    writer: W,
    stack_name: &'a str,
    since: DateTime<Utc>,
    seen_events: HashSet<String>,
    latest_event: Option<DateTime<Utc>>,
}

fn event_sort_key(a: &StackEvent, b: &StackEvent) -> std::cmp::Ordering {
    let a_timestamp = DateTime::parse_from_rfc3339(&a.timestamp).unwrap();
    let b_timestamp = DateTime::parse_from_rfc3339(&b.timestamp).unwrap();

    a_timestamp.partial_cmp(&b_timestamp).unwrap()
}

impl<'a, F, W> Tail<'a, F, W>
where
    F: Fetch + Debug,
    W: WriteColor + Debug,
{
    fn new(fetcher: F, writer: W, stack_name: &'a str, since: DateTime<Utc>) -> Self {
        Self {
            fetcher,
            writer,
            stack_name,
            since,
            seen_events: HashSet::new(),
            latest_event: None,
        }
    }

    // Fetch all of the events since the beginning of time, so that we can ensure all
    // of the events are sorted.
    #[tracing::instrument]
    async fn prefetch(&mut self) {
        let mut all_events = self
            .fetcher
            .fetch_all_events(self.stack_name)
            .await
            .unwrap();
        all_events.sort_by(event_sort_key);

        let last_event = &all_events[all_events.len() - 1];
        self.latest_event = Some(
            DateTime::parse_from_rfc3339(&last_event.timestamp)
                .unwrap()
                .with_timezone(&Utc),
        );
        all_events.iter().for_each(|e| {
            self.seen_events.insert(e.event_id.clone());
        });
    }

    #[tracing::instrument]
    async fn poll(&mut self) -> Result<(), Error> {
        tracing::debug!(start_time = ?self.since, "showing logs from now");

        loop {
            tracing::trace!(seen_events = ?self.seen_events);
            if let Err(e) = self
                .fetcher
                .fetch_events_since(self.stack_name, &self.since)
                .await
                .map(|events| {
                    let mut events = events.clone();
                    tracing::debug!(nevents = events.len(), "found new events");
                    events.sort_by(event_sort_key);
                    for event in events.into_iter() {
                        let timestamp = DateTime::parse_from_rfc3339(&event.timestamp)
                            .expect("parsing event time");
                        // Filter on timestamp
                        if timestamp < self.since {
                            continue;
                        }

                        if self.seen_events.contains(&event.event_id) {
                            continue;
                        }

                        self.print_event(&event);

                        self.seen_events.insert(event.event_id);
                    }
                })
            {
                // TODO: Handle credential refreshing
                if let Some(e) = e.downcast_ref::<rusoto_core::HttpDispatchError>() {
                    let message = format!("{}", e);
                    if message.contains("connection closed before message completed") {
                    } else {
                        eprintln!("error: {}", e);
                    }
                }
            }

            delay_for(Duration::from_secs(5)).await;
        }
    }

    #[tracing::instrument]
    fn print_event(&mut self, event: &rusoto_cloudformation::StackEvent) {
        let resource_name = event.logical_resource_id.as_ref().unwrap();
        let status = event.resource_status.as_ref().unwrap();
        let timestamp = &event.timestamp;
        let status_reason = event.resource_status_reason.as_ref();

        write!(self.writer, "{timestamp}: ", timestamp = timestamp).unwrap();
        if resource_name == self.stack_name {
            let mut spec = ColorSpec::new();
            spec.set_fg(Some(Color::Yellow));
            self.writer.set_color(&spec).unwrap();
            write!(self.writer, "{name}", name = resource_name).unwrap();
            self.writer.reset().unwrap();
        } else {
            write!(self.writer, "{name}", name = resource_name).unwrap();
        }

        write!(self.writer, " | ").unwrap();

        match status.as_str() {
            "UPDATE_IN_PROGRESS" | "UPDATE_COMPLETE_CLEANUP_IN_PROGRESS" => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Blue));
                self.writer.set_color(&spec).unwrap();
            }
            "UPDATE_COMPLETE" => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Green));
                self.writer.set_color(&spec).unwrap();
            }
            "UPDATE_FAILED"
            | "UPDATE_ROLLBACK_IN_PROGRESS"
            | "UPDATE_ROLLBACK_COMPLETE"
            | "UPDATE_ROLLBACK_COMPLETE_CLEANUP_IN_PROGRESS" => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(Color::Red));
                self.writer.set_color(&spec).unwrap();
            }
            _ => {}
        }

        write!(self.writer, "{}", status).expect("printing");
        self.writer.reset().unwrap();

        if let Some(reason) = status_reason {
            writeln!(self.writer, " ({reason})", reason = reason).expect("printing");
        } else {
            writeln!(self.writer, "").expect("printing");
        }
    }
}

struct CFClient(CloudFormationClient);

impl Debug for CFClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CloudformationClient")
    }
}

#[async_trait]
impl Fetch for CFClient {
    async fn fetch_events_since<S>(
        &self,
        stack_name: S,
        start_time: &DateTime<Utc>,
    ) -> Result<Vec<StackEvent>, Box<dyn std::error::Error>>
    where
        S: Into<String> + Send,
    {
        let input = DescribeStackEventsInput {
            stack_name: Some(stack_name.into()),
            ..Default::default()
        };

        let response = self.0.describe_stack_events(input).await?;
        Ok(response
            .stack_events
            .unwrap_or_else(|| Vec::new())
            .into_iter()
            .filter(|e| {
                let timestamp = DateTime::parse_from_rfc3339(&e.timestamp).unwrap();
                &timestamp > start_time
            })
            .collect())
    }

    #[tracing::instrument]
    async fn fetch_all_events<S>(
        &self,
        stack_name: S,
    ) -> Result<Vec<StackEvent>, Box<dyn std::error::Error>>
    where
        S: Into<String> + Send + Debug,
    {
        let mut events = Vec::new();
        let mut next_token: Option<String> = None;
        let stack_name = stack_name.into();
        loop {
            tracing::debug!(next_token = ?next_token, "fetching more events");
            let input = DescribeStackEventsInput {
                stack_name: Some(stack_name.clone()),
                next_token: next_token.clone(),
            };

            match self
                .0
                .describe_stack_events(input)
                .instrument(tracing::debug_span!("fetching events"))
                .await
            {
                Ok(response) => {
                    match response.stack_events {
                        Some(batch) => {
                            events.extend_from_slice(&batch);
                        }
                        None => {
                            tracing::debug!("reached end of events");
                            break;
                        }
                    }

                    if let Some(new_next_token) = response.next_token {
                        next_token = Some(new_next_token);
                    }
                }
                Err(e) => {
                    tracing::error!(err = ?e, "error fetching all events");
                    break;
                }
            }
        }
        tracing::debug!(nevents = events.len(), "got all past events");
        Ok(events)
    }
}

struct Writer<'a>(termcolor::StandardStreamLock<'a>);

impl<'a> Debug for Writer<'a> {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        w.write_str("writer")
    }
}

impl<'a> std::io::Write for Writer<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl<'a> termcolor::WriteColor for Writer<'a> {
    fn supports_color(&self) -> bool {
        self.0.supports_color()
    }

    fn set_color(&mut self, spec: &ColorSpec) -> std::io::Result<()> {
        self.0.set_color(spec)
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.0.reset()
    }
}

#[tracing::instrument]
fn foo(value: i32) -> i32 {
    tracing::info!("inside foo");
    value
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opts::from_args();

    let since = opts
        .since
        .map(|s| Utc.timestamp(s, 0))
        .unwrap_or_else(|| Utc::now());

    loop {
        let region = Region::default();
        tracing::debug!(region = ?region, "chosen region");

        let client = CloudFormationClient::new(region);

        let stdout = StandardStream::stdout(ColorChoice::Auto);
        let handle = Writer(stdout.lock());

        let mut tail = Tail::new(CFClient(client), handle, &opts.stack_name, since);
        // tail.prefetch().await;

        match tail.poll().await {
            Ok(_) => unreachable!(),
            Err(Error::CredentialTimeout) => continue,
            Err(Error::Other(e)) => panic!("{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn something() {
        struct FakeFetcher;

        #[async_trait]
        impl Fetch for FakeFetcher {
            async fn fetch_events_since<S>(
                &self,
                stack_name: S,
                start_time: &DateTime<Utc>,
            ) -> Result<Vec<StackEvent>, Box<dyn std::error::Error>>
            where
                S: Into<String> + Send,
            {
                todo!()
            }
            async fn fetch_all_events<S>(
                &self,
                stack_name: S,
            ) -> Result<Vec<StackEvent>, Box<dyn std::error::Error>>
            where
                S: Into<String> + Send + Debug,
            {
                todo!()
            }
        }

        let writer = StandardStream::stdout(ColorChoice::Auto);
        let handle = writer.lock();

        // let mut tail = Tail::new(FakeFetcher {}, handle);
    }
}
