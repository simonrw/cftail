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

// macro_rules! send_request_with_retry {
//     ($name:literal, $builder:ident, $err:ident) => {
//         backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
//             let name = $name;
//             // any errors that deserve a retry should be wrapped in a `backoff::Error::Temporary`
//             // type so that the retry behaviour kicks in. Other types of errors should be
//             // `backoff::Error::Permanent` to indicate that the failure should not be retried.
//             $builder.clone().send().await.map_err(|e| match e {
//                 aws_sdk_cloudformation::error::SdkError::TimeoutError(_) => {
//                     tracing::trace!(%name, "timeout error, retrying");
//                     backoff::Error::transient($err::Timeout)
//                 }
//                 e => {
//                     match e.code() {
//                         Some(code) if code == "Throttling" => {
//                             tracing::trace!(%name, "throttling error, retrying");
//                             backoff::Error::transient($err::Throttling)
//                         },
//                         _ => backoff::Error::permanent($err::Unknown(e.to_string())),

//                     }
//                 }
//             })
//         })
//         .await
//         .map(From::from)
//         .map_err(From::from)
//     };
// }

#[async_trait::async_trait]
impl AwsCloudFormationClient for Client {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> Result<DescribeStacksOutput, SdkError<DescribeStacksError, HttpResponse>> {
        let builder = Client::describe_stacks(self).stack_name(input.stack_name.unwrap());
        let builder = builder.set_next_token(input.next_token);
        // TODO: retries
        builder.send().await
    }

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> Result<DescribeStackEventsOutput, SdkError<DescribeStackEventsError, HttpResponse>> {
        let builder = Client::describe_stack_events(self).stack_name(input.stack_name.unwrap());
        let builder = builder.set_next_token(input.next_token);
        // TODO: retries
        builder.send().await
    }

    async fn describe_stack_resources(
        &self,
        input: DescribeStackResourcesInput,
    ) -> Result<DescribeStackResourcesOutput, SdkError<DescribeStackResourcesError, HttpResponse>>
    {
        let builder = Client::describe_stack_resources(self).stack_name(input.stack_name.unwrap());
        builder.send().await
    }
}
