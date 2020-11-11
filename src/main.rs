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

#[derive(StructOpt)]
struct Opts {
    stack_name: String,
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

struct Tail<F, W> {
    fetcher: F,
    writer: W,
}

impl<F, W> Tail<F, W>
where
    F: Fetch,
    W: WriteColor,
{
    fn new(fetcher: F, writer: W) -> Self {
        Self { fetcher, writer }
    }

    async fn poll(&mut self, stack_name: &str) {
        let start_time: DateTime<Utc> = Utc::now();

        let mut seen_events = HashSet::new();

        loop {
            if let Err(e) = self
                .fetcher
                .fetch_events_since(stack_name, &start_time)
                .await
                .map(|events| {
                    for event in events.into_iter().rev() {
                        let timestamp = DateTime::parse_from_rfc3339(&event.timestamp)
                            .expect("parsing event time");
                        // Filter on timestamp
                        if timestamp < start_time {
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
            .rev()
            .collect())
    }
}

#[tokio::main]
async fn main() {
    let opts = Opts::from_args();

    let region = Region::default();
    let client = CloudFormationClient::new(region);

    let stdout = StandardStream::stdout(ColorChoice::Always);
    let handle = stdout.lock();

    let mut tail = Tail::new(CFClient(client), handle);
    tail.poll(&opts.stack_name).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn something() {
        assert!(true);
    }
}
