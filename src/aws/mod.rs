mod rusoto;

use rusoto_core::RusotoError;

pub(crate) type AwsResult<T, E> = Result<T, RusotoError<E>>;

/// Trait representing interactions with CloudFormation
#[async_trait::async_trait]
pub(crate) trait AwsCloudFormationClient {
    async fn describe_stacks(
        &self,
        input: rusoto_cloudformation::DescribeStacksInput,
    ) -> AwsResult<DescribeStacksOutput, rusoto_cloudformation::DescribeStacksError>;

    async fn describe_stack_events(
        &self,
        input: rusoto_cloudformation::DescribeStackEventsInput,
    ) -> AwsResult<
        rusoto_cloudformation::DescribeStackEventsOutput,
        rusoto_cloudformation::DescribeStackEventsError,
    >;
}

pub(crate) struct Output {
    pub(crate) key: String,
    pub(crate) value: String,
}

pub(crate) struct Stack {
    pub(crate) outputs: Option<Vec<Output>>,
}

pub(crate) struct DescribeStacksOutput {
    pub(crate) stacks: Vec<Stack>,
}
