use async_trait::async_trait;
use chrono::prelude::*;
use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsInput, StackEvent,
};
use rusoto_core::Region;
use std::collections::HashSet;
use std::time::Duration;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::time::delay_for;
use std::fmt::Debug;

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
}

#[derive(Debug)]
struct Tail<F, W> {
    fetcher: F,
    writer: W,
}

impl<F, W> Tail<F, W>
where
    F: Fetch + Debug,
    W: WriteColor + Debug,
{
    fn new(fetcher: F, writer: W) -> Self {
        Self { fetcher, writer }
    }

    #[tracing::instrument]
    async fn poll(&mut self, stack_name: &str, since: DateTime<Utc>) {
        tracing::debug!(start_time = ?since, "showing logs from now");

        let mut seen_events = HashSet::new();

        loop {
            tracing::trace!(seen_events = ?seen_events);
            if let Err(e) = self
                .fetcher
                .fetch_events_since(stack_name, &since)
                .await
                .map(|events| {
                    let mut events = events.clone();
                    tracing::debug!(nevents = events.len(), "found new events");
                    events.sort_by(|a, b| {
                        let a_timestamp = DateTime::parse_from_rfc3339(&a.timestamp).unwrap();
                        let b_timestamp = DateTime::parse_from_rfc3339(&b.timestamp).unwrap();

                        a_timestamp.partial_cmp(&b_timestamp).unwrap()
                    });
                    for event in events.into_iter() {
                        let timestamp = DateTime::parse_from_rfc3339(&event.timestamp)
                            .expect("parsing event time");
                        // Filter on timestamp
                        if timestamp < since {
                            continue;
                        }

                        if seen_events.contains(&event.event_id) {
                            continue;
                        }

                        self.print_event(&event);

                        seen_events.insert(event.event_id);
                    }
                })
            {
                eprintln!("error requesting stack events: {:?}", e);
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
        write!(
            self.writer,
            "{timestamp}: {name} | ",
            timestamp = timestamp,
            name = resource_name
        )
        .expect("printing");
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

    let since = opts.since.map(|s| Utc.timestamp(s, 0)).unwrap_or_else(|| Utc::now());

    let region = Region::default();
    tracing::debug!(region = ?region, "chosen region");
    let client = CloudFormationClient::new(region);

    let stdout = StandardStream::stdout(ColorChoice::Auto);
    let handle = Writer(stdout.lock());

    let mut tail = Tail::new(CFClient(client), handle);
    tail.poll(&opts.stack_name, since).await;
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
        }

        let writer = StandardStream::stdout(ColorChoice::Auto);
        let handle = writer.lock();

        let mut tail = Tail::new(FakeFetcher {}, handle);
    }
}
