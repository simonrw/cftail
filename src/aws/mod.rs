mod aws_sdk;


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

    async fn describe_stack_resources(
        &self,
        input: DescribeStackResourcesInput,
    ) -> Result<DescribeStackResourcesOutput, DescribeStackResourcesError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DescribeStacksError {
    #[error("request timeout")]
    Timeout,
    #[error("request was throttled")]
    Throttling,
    #[error("unknown error: {0}")]
    Unknown(String),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub(crate) enum DescribeStackEventsError {
    #[error("request timeout")]
    Timeout,
    #[error("request was throttled")]
    Throttling,
    #[error("unknown error: {0}")]
    Unknown(String),
    #[error("error dispatching request")]
    Dispatch,
    #[error("error with respone")]
    Response,
    #[error("service error")]
    Service,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DescribeStackResourcesError {
    #[error("request timeout")]
    Timeout,
    #[error("request was throttled")]
    Throttling,
    #[error("unknown error: {0}")]
    Unknown(String),
}

// DescribeStacks

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

// DescribeStackEvents

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

// DescribeStackResources

#[derive(Debug, Default)]
pub(crate) struct DescribeStackResourcesInput {
    pub(crate) stack_name: String,
}

pub(crate) struct DescribeStackResourcesOutput {
    pub(crate) stack_resources: Vec<StackResource>,
}

pub(crate) struct StackResource {
    pub(crate) resource_type: String,
    pub(crate) physical_resource_id: Option<String>,
    pub(crate) stack_name: String,
}
