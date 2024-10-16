use super::AwsCloudFormationClient;

use aws_sdk_cloudformation::config::http::HttpResponse;
use aws_sdk_cloudformation::error::SdkError;
use aws_sdk_cloudformation::operation::describe_stack_events::{
    DescribeStackEventsError, DescribeStackEventsInput, DescribeStackEventsOutput,
};
use aws_sdk_cloudformation::operation::describe_stack_resources::{
    DescribeStackResourcesError, DescribeStackResourcesInput, DescribeStackResourcesOutput,
};
use aws_sdk_cloudformation::operation::describe_stacks::{
    DescribeStacksError, DescribeStacksInput, DescribeStacksOutput,
};
use aws_sdk_cloudformation::Client;
use backoff::ExponentialBackoff;

macro_rules! send_request_with_retry {
    ($builder:ident) => {{
        use aws_smithy_types::error::metadata::ProvideErrorMetadata;

        backoff::future::retry(ExponentialBackoff::default(), || async {
            $builder.clone().send().await.map_err(|e| match e {
                err @ SdkError::TimeoutError(_) => backoff::Error::transient(err),
                err => match err.code() {
                    Some(code) if code == "Throttling" => backoff::Error::transient(err),
                    _ => backoff::Error::permanent(err),
                },
            })
        })
        .await
    }};
}

#[async_trait::async_trait]
impl AwsCloudFormationClient for Client {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> Result<DescribeStacksOutput, SdkError<DescribeStacksError, HttpResponse>> {
        let builder = Client::describe_stacks(self).stack_name(input.stack_name.unwrap());
        let builder = builder.set_next_token(input.next_token);
        send_request_with_retry!(builder)
    }

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> Result<DescribeStackEventsOutput, SdkError<DescribeStackEventsError, HttpResponse>> {
        let builder = Client::describe_stack_events(self).stack_name(input.stack_name.unwrap());
        let builder = builder.set_next_token(input.next_token);
        send_request_with_retry!(builder)
    }

    async fn describe_stack_resources(
        &self,
        input: DescribeStackResourcesInput,
    ) -> Result<DescribeStackResourcesOutput, SdkError<DescribeStackResourcesError, HttpResponse>>
    {
        let builder = Client::describe_stack_resources(self).stack_name(input.stack_name.unwrap());
        send_request_with_retry!(builder)
    }
}
