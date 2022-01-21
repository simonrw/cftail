use rusoto_core::RusotoError;

mod rusoto;

pub(crate) type AwsResult<T, E> = Result<T, RusotoError<E>>;

/// Trait representing interactions with CloudFormation
#[async_trait::async_trait]
pub(crate) trait AwsCloudFormationClient {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> AwsResult<DescribeStacksOutput, rusoto_cloudformation::DescribeStacksError>;

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> AwsResult<DescribeStackEventsOutput, rusoto_cloudformation::DescribeStackEventsError>;
}

#[derive(Default)]
pub(crate) struct DescribeStacksInput {
    pub(crate) stack_name: Option<String>,
    pub(crate) next_token: Option<String>,
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

#[derive(Debug, Default)]
pub(crate) struct DescribeStackEventsInput {
    pub(crate) stack_name: Option<String>,
    pub(crate) next_token: Option<String>,
}

pub(crate) struct DescribeStackEventsOutput {
    pub(crate) next_token: Option<String>,
    pub(crate) stack_events: Vec<StackEvent>,
}

pub(crate) struct StackEvent {
    pub(crate) timestamp: String,
    pub(crate) logical_resource_id: Option<String>,
    pub(crate) resource_status: Option<String>,
    pub(crate) stack_name: String,
    pub(crate) resource_status_reason: Option<String>,
}
