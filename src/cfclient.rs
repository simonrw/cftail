use crate::error::Error;
use crate::fetch::Fetch;
use async_trait::async_trait;
use chrono::prelude::*;
use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsError, DescribeStackEventsInput,
    StackEvent,
};
use rusoto_core::RusotoError;
use std::fmt::Debug;
use tracing::Instrument;

pub(crate) struct CFClient(CloudFormationClient);

impl CFClient {
    pub(crate) fn new(inner: CloudFormationClient) -> Self {
        Self(inner)
    }
}

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
    ) -> Result<Vec<StackEvent>, Error>
    where
        S: Into<String> + Send,
    {
        let input = DescribeStackEventsInput {
            stack_name: Some(stack_name.into()),
            ..Default::default()
        };

        let response = self
            .0
            .describe_stack_events(input)
            .await
            .map_err(Error::Rusoto)?;

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

    // #[tracing::instrument]
    async fn fetch_all_events<S>(&self, stack_name: S) -> Result<Vec<StackEvent>, Error>
    where
        S: Into<String> + Send + Debug,
    {
        async move {
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
                        tracing::debug!("got successful response");
                        match response.stack_events {
                            Some(batch) => {
                                events.extend_from_slice(&batch);
                            }
                            None => {
                                tracing::debug!("reached end of events");
                                break;
                            }
                        }

                        match response.next_token {
                            Some(new_next_token) => next_token = Some(new_next_token),
                            None => break,
                        }
                    }
                    Err(e) => {
                        tracing::debug!("got failed response");
                        match e {
                            RusotoError::Service(ref error) => {
                                tracing::error!(err = %error, "rusoto error");
                                return Err(Error::Rusoto(e));
                            }
                            _ => tracing::error!(err = ?e, "error"),
                        }
                        break;
                    }
                }
            }
            tracing::debug!(nevents = events.len(), "got all past events");
            Ok(events)
        }
        .instrument(tracing::debug_span!("fetching events"))
        .await
    }
}
