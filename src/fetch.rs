use crate::error::Error;
use async_trait::async_trait;
use chrono::prelude::*;
use rusoto_cloudformation::StackEvent;
use std::fmt::Debug;

#[async_trait]
pub(crate) trait Fetch {
    async fn fetch_events_since<S>(
        &self,
        stack_name: S,
        start_time: &DateTime<Utc>,
    ) -> Result<Vec<StackEvent>, Error>
    where
        S: Into<String> + Send;

    async fn fetch_all_events<S>(&self, stack_name: S) -> Result<Vec<StackEvent>, Error>
    where
        S: Into<String> + Send + Debug;
}
