use chrono::prelude::*;
use rusoto_cloudformation::CloudFormationClient;
use rusoto_core::Region;
use structopt::StructOpt;
use termcolor::{ColorChoice, StandardStream};

mod cfclient;
mod error;
mod exponential_backoff;
mod fetch;
mod tail;
mod writer;

use error::Error;
use tail::Tail;
use writer::Writer;

#[derive(StructOpt)]
struct Opts {
    stack_name: String,

    #[structopt(short, long)]
    since: Option<i64>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opts::from_args();
    let since = opts
        .since
        .map(|s| Utc.timestamp(s, 0))
        .unwrap_or_else(|| Utc::now());

    tracing::info!(stack_name = %opts.stack_name, since = %since, "tailing stack events");

    loop {
        let region = Region::default();
        tracing::debug!(region = ?region, "chosen region");

        let client = CloudFormationClient::new(region);

        let stdout = StandardStream::stdout(ColorChoice::Auto);
        let handle = Writer::new(stdout.lock());

        let mut tail = Tail::new(
            cfclient::CFClient::new(client),
            handle,
            &opts.stack_name,
            since,
        );
        // tail.prefetch().await;

        match tail.poll().await {
            Ok(_) => unreachable!(),
            Err(Error::CredentialTimeout) => continue,
            Err(Error::Http(_r)) => break,
            Err(Error::Other(e)) => panic!("{}", e),
        }
    }
}
