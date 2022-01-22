use super::{
    AwsCloudFormationClient, DescribeStackEventsError, DescribeStackEventsInput,
    DescribeStackEventsOutput, DescribeStackResourcesError, DescribeStackResourcesInput,
    DescribeStackResourcesOutput, DescribeStacksError, DescribeStacksInput, DescribeStacksOutput,
    Output, Stack, StackEvent, StackResource,
};
use rusoto_cloudformation::{CloudFormation, CloudFormationClient};
use rusoto_core::RusotoError;

pub(crate) type AwsResult<T, E> = Result<T, RusotoError<E>>;

#[async_trait::async_trait]
impl AwsCloudFormationClient for CloudFormationClient {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> Result<DescribeStacksOutput, DescribeStacksError> {
        CloudFormation::describe_stacks(self, input.into())
            .await
            .map(From::from)
            .map_err(From::from)
    }

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> Result<DescribeStackEventsOutput, DescribeStackEventsError> {
        CloudFormation::describe_stack_events(self, input.into())
            .await
            .map(From::from)
            .map_err(From::from)
    }

    async fn describe_stack_resources(
        &self,
        input: DescribeStackResourcesInput,
    ) -> Result<DescribeStackResourcesOutput, DescribeStackResourcesError> {
        CloudFormation::describe_stack_resources(self, input.into())
            .await
            .map(From::from)
            .map_err(From::from)
    }
}

// conversions to and from third party types

impl From<rusoto_core::RusotoError<rusoto_cloudformation::DescribeStacksError>>
    for DescribeStacksError
{
    fn from(_: rusoto_core::RusotoError<rusoto_cloudformation::DescribeStacksError>) -> Self {
        todo!()
    }
}

impl From<rusoto_core::RusotoError<rusoto_cloudformation::DescribeStackEventsError>>
    for DescribeStackEventsError
{
    fn from(_: rusoto_core::RusotoError<rusoto_cloudformation::DescribeStackEventsError>) -> Self {
        todo!()
    }
}

impl From<rusoto_core::RusotoError<rusoto_cloudformation::DescribeStackResourcesError>>
    for DescribeStackResourcesError
{
    fn from(
        _: rusoto_core::RusotoError<rusoto_cloudformation::DescribeStackResourcesError>,
    ) -> Self {
        todo!()
    }
}
impl From<DescribeStacksInput> for rusoto_cloudformation::DescribeStacksInput {
    fn from(i: DescribeStacksInput) -> Self {
        Self {
            stack_name: i.stack_name.clone(),
            next_token: i.next_token.clone(),
        }
    }
}

impl From<DescribeStackEventsInput> for rusoto_cloudformation::DescribeStackEventsInput {
    fn from(i: DescribeStackEventsInput) -> Self {
        Self {
            next_token: i.next_token.clone(),
            stack_name: i.stack_name.clone(),
        }
    }
}

impl From<&rusoto_cloudformation::Output> for Output {
    fn from(o: &rusoto_cloudformation::Output) -> Self {
        Self {
            key: o.output_key.as_ref().unwrap().to_string(),
            value: o.output_value.as_ref().unwrap().to_string(),
        }
    }
}

impl From<&rusoto_cloudformation::Stack> for Stack {
    fn from(s: &rusoto_cloudformation::Stack) -> Self {
        Self {
            outputs: s
                .outputs
                .as_ref()
                .map(|o| o.iter().map(From::from).collect()),
        }
    }
}

impl From<rusoto_cloudformation::DescribeStacksOutput> for DescribeStacksOutput {
    fn from(source: rusoto_cloudformation::DescribeStacksOutput) -> Self {
        Self {
            stacks: source
                .stacks
                .unwrap_or_else(Vec::new)
                .iter()
                .map(From::from)
                .collect(),
        }
    }
}

impl From<&rusoto_cloudformation::StackEvent> for StackEvent {
    fn from(e: &rusoto_cloudformation::StackEvent) -> Self {
        Self {
            timestamp: e.timestamp.clone(),
            logical_resource_id: e.logical_resource_id.clone(),
            resource_status: e.resource_status.clone(),
            stack_name: e.stack_name.clone(),
            resource_status_reason: e.resource_status_reason.clone(),
        }
    }
}

impl From<rusoto_cloudformation::DescribeStackEventsOutput> for DescribeStackEventsOutput {
    fn from(o: rusoto_cloudformation::DescribeStackEventsOutput) -> Self {
        Self {
            next_token: o.next_token.clone(),
            stack_events: o
                .stack_events
                .unwrap_or_else(Vec::new)
                .iter()
                .map(From::from)
                .collect(),
        }
    }
}

// DescribeStackResources

impl From<DescribeStackResourcesInput> for rusoto_cloudformation::DescribeStackResourcesInput {
    fn from(i: DescribeStackResourcesInput) -> Self {
        Self {
            stack_name: Some(i.stack_name.clone()),
            ..Default::default()
        }
    }
}

impl From<rusoto_cloudformation::DescribeStackResourcesOutput> for DescribeStackResourcesOutput {
    fn from(o: rusoto_cloudformation::DescribeStackResourcesOutput) -> Self {
        Self {
            stack_resources: o
                .stack_resources
                .unwrap_or_else(Vec::new)
                .iter()
                .map(From::from)
                .collect(),
        }
    }
}

impl From<&rusoto_cloudformation::StackResource> for StackResource {
    fn from(r: &rusoto_cloudformation::StackResource) -> Self {
        Self {
            resource_type: r.resource_type.clone(),
            physical_resource_id: r.physical_resource_id.clone(),
            stack_name: r.stack_name.as_ref().unwrap().clone(),
        }
    }
}
