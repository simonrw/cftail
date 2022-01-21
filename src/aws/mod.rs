mod rusoto;

/// Trait representing interactions with CloudFormation
#[async_trait::async_trait]
pub(crate) trait AwsCloudFormationClient {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> Result<DescribeStacksOutput, DescribeStacksError>;

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> Result<DescribeStackEventsOutput, DescribeStackEventsError>;
}

#[derive(Debug)]
pub(crate) struct DescribeStacksError {}

#[derive(Debug)]
pub(crate) struct DescribeStackEventsError {}

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
