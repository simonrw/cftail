use rusoto_cloudformation::{CloudFormation, CloudFormationClient, DescribeStackEventsInput};
use rusoto_core::Region;
use chrono::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::delay_for;

fn print_event<W: std::io::Write>(event: &rusoto_cloudformation::StackEvent, mut writer: W) {
    let resource_name = event.logical_resource_id.as_ref().unwrap();
    let status = event.resource_status.as_ref().unwrap();
    let timestamp = &event.timestamp;
    let status_reason = event.resource_status_reason.as_ref();
    if let Some(reason) = status_reason {
        writeln!(writer, "{timestamp}: {name} | {status} ({reason})", timestamp=timestamp, name=resource_name, status=status, reason=reason).expect("printing");
    } else {
        writeln!(writer, "{timestamp}: {name} | {status}", timestamp=timestamp, name=resource_name, status=status).expect("printing");
    }
}

#[tokio::main]
async fn main() {
    let region = Region::default();
    let client = CloudFormationClient::new(region);

    let input = DescribeStackEventsInput {
        stack_name: Some("stockton-storage-buckets".to_string()),
        ..Default::default()
    };

    let _start_time: DateTime<Utc> = Utc::now();

    let mut seen_events = HashSet::new();

    loop {
        let response = client
            .describe_stack_events(input.clone())
            .await
            .expect("describing stack events");

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        if let Some(events) = response.stack_events {
            for event in events {
                let _timestamp = DateTime::parse_from_rfc3339(&event.timestamp).expect("parsing event time");
                // Filter on timestamp
                // if timestamp < start_time {
                //     continue;
                // }

                if seen_events.contains(&event.event_id) {
                    continue;
                }

                print_event(&event, &mut handle);
                
                seen_events.insert(event.event_id);

            }
        }

        delay_for(Duration::from_secs(5)).await;
    }
}