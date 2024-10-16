use aws_sdk_cloudformation::{
    config::http::HttpResponse,
    error::SdkError,
    operation::{
        describe_stack_events::{
            DescribeStackEventsError, DescribeStackEventsInput, DescribeStackEventsOutput,
        },
        describe_stack_resources::{
            DescribeStackResourcesError, DescribeStackResourcesInput, DescribeStackResourcesOutput,
        },
        describe_stacks::{DescribeStacksError, DescribeStacksInput, DescribeStacksOutput},
    },
};

mod aws_sdk;

/// Trait representing interactions with CloudFormation
#[async_trait::async_trait]
pub(crate) trait AwsCloudFormationClient {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> Result<DescribeStacksOutput, SdkError<DescribeStacksError, HttpResponse>>;

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> Result<DescribeStackEventsOutput, SdkError<DescribeStackEventsError, HttpResponse>>;

    async fn describe_stack_resources(
        &self,
        input: DescribeStackResourcesInput,
    ) -> Result<DescribeStackResourcesOutput, SdkError<DescribeStackResourcesError, HttpResponse>>;
}
