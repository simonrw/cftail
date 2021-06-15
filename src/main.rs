use chrono::{prelude::*, Duration as ChronoDuration};
use eyre::{Result, WrapErr};
use rusoto_cloudformation::CloudFormationClient;
use rusoto_core::Region;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;
use termcolor::{ColorChoice, StandardStream};
use tokio::time::delay_for;

mod error;
mod nested_stacks;
mod stack_status;
mod tail;
mod writer;

use crate::error::Error;
use crate::tail::Tail;
use crate::writer::Writer;

// Custom parser for parsing the datetime as either a timestamp, or as a handy string.
fn parse_since_argument(src: &str) -> Result<DateTime<Utc>> {
    // Try to parse as datetime
    if let Ok(dt) = DateTime::from_str(src) {
        return Ok(dt);
    }

    // Try to parse as naive datetime (and assume UTC)
    if let Ok(dt) = NaiveDateTime::from_str(src).map(|n| DateTime::<Utc>::from_utc(n, Utc)) {
        return Ok(dt);
    }

    // Try to parse as timestamp
    if let Ok(dt) = src.parse::<i64>().map(|i| Utc.timestamp(i, 0)) {
        return Ok(dt);
    }

    // some common terms
    if src == "today" {
        let today = Utc::today();
        let dt = today.and_hms(0, 0, 0);
        return Ok(dt);
    } else if src == "yesterday" {
        let yesterday = Utc::today() - ChronoDuration::days(1);
        let dt = yesterday.and_hms(0, 0, 0);
        return Ok(dt);
    }

    Err(Error::ParseSince).wrap_err("error parsing since argument")
}

#[derive(StructOpt)]
struct Opts {
    stack_name: String,

    #[structopt(short, long, parse(try_from_str = parse_since_argument))]
    since: Option<DateTime<Utc>>,

    #[structopt(short, long)]
    nested: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    color_eyre::install().unwrap();

    let opts = Opts::from_args();
    let since = opts.since.unwrap_or_else(|| Utc::now());

    tracing::info!(stack_name = %opts.stack_name, since = %since, "tailing stack events");

    let mut seen_events = HashSet::new();

    loop {
        let region = Region::default();
        tracing::debug!(region = ?region, "chosen region");

        let client = CloudFormationClient::new(region);
        let stacks = if opts.nested {
            nested_stacks::fetch_nested_stack_names(&client, &opts.stack_name)
                .await
                .expect("fetching nested stacks")
        } else {
            let mut stacks = HashSet::new();
            stacks.insert(opts.stack_name.clone());
            stacks
        };

        let handle = Writer::new();

        let mut tail = Tail::new(Arc::new(client), handle, &stacks, since, &mut seen_events);

        tracing::info!("prefetching tasks");
        match tail.prefetch().await {
            Ok(_) => {}
            Err(e) => match e.downcast_ref::<Error>() {
                Some(Error::NoCredentials) => {
                    eprintln!("Error: no valid credentials found");
                    std::process::exit(1);
                }
                Some(Error::NoStack) => {
                    eprintln!("Error: could not find stack {}", opts.stack_name);
                    std::process::exit(1);
                }
                Some(Error::CredentialsExpired) => {
                    eprintln!("Error: your credentials have expired");
                    std::process::exit(1);
                }
                Some(Error::RateLimitExceeded) => {
                    tracing::warn!("rate limit exceeded");
                    delay_for(Duration::from_secs(5)).await;
                }
                Some(e) => {
                    eprintln!("Error: unknown error: {:?}", e);
                    std::process::exit(1);
                }
                None => {
                    eprintln!("Error: unknown error: {:?}", e);
                    std::process::exit(1);
                }
            },
        }

        tracing::debug!("starting poll loop");
        // match tail.poll().await {
        //     Ok(_) => unreachable!(),
        //     Err(e) => match e.downcast_ref::<Error>() {
        //         Some(Error::CredentialsExpired) => {
        //             eprintln!("Error: your credentials have expired");
        //             std::process::exit(1);
        //         }
        //         Some(Error::RateLimitExceeded) => {
        //             tracing::warn!("rate limit exceeded");
        //             delay_for(Duration::from_secs(5)).await;
        //         }
        //         Some(e) => {
        //             tracing::error!(err = %e, "unexpected error");
        //             std::process::exit(1);
        //         }
        //         None => {
        //             tracing::error!(err = %e, "unexpected error");
        //             std::process::exit(1);
        //         }
        //     },
        // }

        tracing::trace!("building another client");
    }
}
