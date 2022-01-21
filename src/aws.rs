use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsError, DescribeStackEventsInput,
    DescribeStackEventsOutput, DescribeStacksError, DescribeStacksInput, DescribeStacksOutput,
};
use rusoto_core::RusotoError;

pub(crate) type AwsResult<T, E> = Result<T, RusotoError<E>>;

/// Trait representing interactions with CloudFormation
#[async_trait::async_trait]
pub(crate) trait AwsCloudFormationClient {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> AwsResult<DescribeStacksOutput, DescribeStacksError>;

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> AwsResult<DescribeStackEventsOutput, DescribeStackEventsError>;
}

#[async_trait::async_trait]
impl AwsCloudFormationClient for CloudFormationClient {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> AwsResult<DescribeStacksOutput, DescribeStacksError> {
        CloudFormation::describe_stacks(self, input).await
    }

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> AwsResult<DescribeStackEventsOutput, DescribeStackEventsError> {
        CloudFormation::describe_stack_events(self, input).await
    }
}
