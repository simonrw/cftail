use async_trait::async_trait;
use chrono::prelude::*;
use rusoto_cloudformation::{DescribeStackEventsError, StackEvent};
use rusoto_core::RusotoError;
use std::fmt::Debug;

#[async_trait]
pub(crate) trait Fetch {
    async fn fetch_events_since<S>(
        &self,
        stack_name: S,
        start_time: &DateTime<Utc>,
    ) -> Result<Vec<StackEvent>, RusotoError<DescribeStackEventsError>>
    where
        S: Into<String> + Send;

    async fn fetch_all_events<S>(
        &self,
        stack_name: S,
    ) -> Result<Vec<StackEvent>, RusotoError<DescribeStackEventsError>>
    where
        S: Into<String> + Send + Debug;
}
