use super::{AwsCloudFormationClient, DescribeStacksInput, DescribeStacksOutput, Output, Stack};
use rusoto_cloudformation::{
    CloudFormation, CloudFormationClient, DescribeStackEventsError, DescribeStackEventsInput,
    DescribeStackEventsOutput, DescribeStacksError,
};
use rusoto_core::RusotoError;

pub(crate) type AwsResult<T, E> = Result<T, RusotoError<E>>;

#[async_trait::async_trait]
impl AwsCloudFormationClient for CloudFormationClient {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> AwsResult<DescribeStacksOutput, DescribeStacksError> {
        CloudFormation::describe_stacks(self, input.into())
            .await
            .map(From::from)
    }

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> AwsResult<DescribeStackEventsOutput, DescribeStackEventsError> {
        CloudFormation::describe_stack_events(self, input).await
    }
}

// conversions to and from third party types

impl From<DescribeStacksInput> for rusoto_cloudformation::DescribeStacksInput {
    fn from(i: DescribeStacksInput) -> Self {
        Self {
            stack_name: i.stack_name.clone(),
            next_token: i.next_token.clone(),
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
