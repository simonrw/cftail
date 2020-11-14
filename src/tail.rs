use crate::error::Error;
use crate::exponential_backoff::backoff;
use crate::fetch::Fetch;
use chrono::{DateTime, Utc};
use rusoto_cloudformation::StackEvent;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::time::Duration;
use termcolor::{Color, ColorSpec, WriteColor};
use tokio::time::delay_for;
use tracing::Instrument;

fn event_sort_key(a: &StackEvent, b: &StackEvent) -> std::cmp::Ordering {
    let a_timestamp = DateTime::parse_from_rfc3339(&a.timestamp).unwrap();
    let b_timestamp = DateTime::parse_from_rfc3339(&b.timestamp).unwrap();

    a_timestamp.partial_cmp(&b_timestamp).unwrap()
}

#[derive(Debug)]
pub(crate) struct Tail<'a, F, W> {
    fetcher: F,
    writer: W,
    stack_name: &'a str,
    since: DateTime<Utc>,
    seen_events: HashSet<String>,
    latest_event: Option<DateTime<Utc>>,
}

impl<'a, F, W> Tail<'a, F, W>
where
    F: Fetch + Debug,
    W: WriteColor + Debug,
{
    pub(crate) fn new(fetcher: F, writer: W, stack_name: &'a str, since: DateTime<Utc>) -> Self {
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
    async fn prefetch<E>(&mut self) {
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

    pub(crate) async fn poll(&mut self) -> Result<(), Error> {
        tracing::debug!(start_time = ?self.since, "showing logs from now");

        async move {
            loop {
                let res = backoff(5, || {
                    self.fetcher
                        .fetch_events_since(self.stack_name, &self.since)
                })
                .instrument(tracing::trace_span!("backoff"))
                .await;

                let res = res.map(|events| {
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
                });

                if let Err(e) = res {
                    match e {
                        rusoto_core::RusotoError::Unknown(response) => {
                            // TODO: Handle credential refreshing
                            tracing::warn!(
                                status_code = response.status.as_u16(),
                                message = response.body_as_str(),
                                "HTTP error"
                            );
                            return Err(Error::Http(response));
                        }
                        _ => tracing::warn!(err = ?e, "unexpected error"),
                    }
                }

                tracing::trace!("sleeping");
                delay_for(Duration::from_secs(5)).await;
            }
        }
        .instrument(tracing::debug_span!("poll-loop"))
        .await
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

        let stack_status = crate::stack_status::StackStatus::try_from(status.as_str())
            .expect("unhandled stack status");
        if let Some(spec) = stack_status.color_spec() {
            self.writer.set_color(&spec).unwrap();
        }

        write!(self.writer, "{}", status).expect("printing");
        self.writer.reset().unwrap();

        if let Some(reason) = status_reason {
            writeln!(self.writer, " ({reason})", reason = reason).expect("printing");
        } else {
            if stack_status.is_complete() && resource_name == self.stack_name {
                writeln!(self.writer, " 🎉✨🤘").expect("printing");
            } else {
                writeln!(self.writer, "").expect("printing");
            }
        }
    }
}
