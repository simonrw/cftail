use chrono::prelude::*;
use rusoto_cloudformation::CloudFormationClient;
use rusoto_core::Region;
use std::collections::HashSet;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;
use termcolor::{ColorChoice, StandardStream};
use tokio::time::delay_for;

mod error;
mod exponential_backoff;
mod stack_status;
mod tail;
mod writer;

use error::Error;
use tail::Tail;
use writer::Writer;

// Custom parser for parsing the datetime as either a timestamp, or as a handy string.
fn parse_since_argument(src: &str) -> Result<DateTime<Utc>, Error> {
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

    Err(Error::ParseSince)
}

#[derive(StructOpt)]
struct Opts {
    stack_name: String,

    #[structopt(short, long, parse(try_from_str = parse_since_argument))]
    since: Option<DateTime<Utc>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opts::from_args();
    let since = opts.since.unwrap_or_else(|| Utc::now());

    tracing::info!(stack_name = %opts.stack_name, since = %since, "tailing stack events");

    let mut seen_events = HashSet::new();

    loop {
        let region = Region::default();
        tracing::debug!(region = ?region, "chosen region");

        let client = CloudFormationClient::new(region);

        let stdout = StandardStream::stdout(ColorChoice::Auto);
        let handle = Writer::new(stdout.lock());

        let mut tail = Tail::new(&client, handle, &opts.stack_name, since, &mut seen_events);

        match tail.prefetch().await {
            Ok(_) => {}
            Err(Error::Aws(error::AwsError::CredentialExpired)) => {
                eprintln!("Your credentials have expired");
                std::process::exit(1);
            }
            Err(Error::Aws(error::AwsError::NoCredentials)) => {
                eprintln!("No valid credentials found");
                std::process::exit(1);
            }
            Err(Error::Aws(error::AwsError::NoStack)) => {
                eprintln!("could not find stack {}", opts.stack_name);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("unknown error: {:?}", e);
                std::process::exit(1);
            }
        }

        tracing::debug!("starting poll loop");
        match tail.poll().await {
            Ok(_) => unreachable!(),
            Err(Error::Aws(error::AwsError::RateLimitExceeded)) => {
                delay_for(Duration::from_secs(5)).await;
            }
            Err(Error::Http(r)) => {
                tracing::error!(status_code = %r.status, message = %r.body_as_str(), "error making request");
                std::process::exit(1);
            }
            Err(e) => {
                tracing::error!(err = %e, "unexpected error");
                std::process::exit(1);
            }
        }

        tracing::trace!("building another client");
    }
}
