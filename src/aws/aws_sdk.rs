use super::{
    AwsCloudFormationClient, DescribeStackEventsError, DescribeStackEventsInput,
    DescribeStackEventsOutput, DescribeStackResourcesError, DescribeStackResourcesInput,
    DescribeStackResourcesOutput, DescribeStacksError, DescribeStacksInput, DescribeStacksOutput,
    Output, Stack, StackEvent, StackResource,
};

use aws_sdk_cloudformation::Client;
use aws_smithy_types::date_time::Format;

macro_rules! send_request_with_retry {
    ($name:literal, $builder:ident, $err:ident) => {
        backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
            let name = $name;
            // any errors that deserve a retry should be wrapped in a `backoff::Error::Temporary`
            // type so that the retry behaviour kicks in. Other types of errors should be
            // `backoff::Error::Permanent` to indicate that the failure should not be retried.
            $builder.clone().send().await.map_err(|e| match e {
                aws_sdk_cloudformation::types::SdkError::TimeoutError(_) => {
                    tracing::trace!(%name, "timeout error, retrying");
                    backoff::Error::transient($err::Timeout)
                }
                aws_sdk_cloudformation::types::SdkError::ServiceError { err, .. } => match err.code() {
                    Some(code) if code == "Throttling" => {
                        tracing::trace!(%name, "throttling error, retrying");
                        backoff::Error::transient($err::Throttling)
                    }

                    _ => backoff::Error::permanent($err::Unknown(err.to_string())),
                },
                _ => backoff::Error::permanent($err::Unknown(e.to_string())),
            })
        })
        .await
        .map(From::from)
        .map_err(From::from)
    };
}

#[async_trait::async_trait]
impl AwsCloudFormationClient for Client {
    async fn describe_stacks(
        &self,
        input: DescribeStacksInput,
    ) -> Result<DescribeStacksOutput, DescribeStacksError> {
        let builder = Client::describe_stacks(self).stack_name(input.stack_name.unwrap());
        let builder = builder.set_next_token(input.next_token);
        send_request_with_retry!("describe_stacks", builder, DescribeStacksError)
    }

    async fn describe_stack_events(
        &self,
        input: DescribeStackEventsInput,
    ) -> Result<DescribeStackEventsOutput, DescribeStackEventsError> {
        let builder = Client::describe_stack_events(self).stack_name(input.stack_name.unwrap());
        let builder = builder.set_next_token(input.next_token);
        send_request_with_retry!("describe_stack_events", builder, DescribeStackEventsError)
    }

    async fn describe_stack_resources(
        &self,
        input: DescribeStackResourcesInput,
    ) -> Result<DescribeStackResourcesOutput, DescribeStackResourcesError> {
        let builder = Client::describe_stack_resources(self).stack_name(input.stack_name);
        send_request_with_retry!(
            "describe_stack_resources",
            builder,
            DescribeStackResourcesError
        )
    }
}

impl From<aws_sdk_cloudformation::output::DescribeStackEventsOutput> for DescribeStackEventsOutput {
    fn from(o: aws_sdk_cloudformation::output::DescribeStackEventsOutput) -> Self {
        Self {
            next_token: o.next_token,
            stack_events: o
                .stack_events
                .unwrap_or_default()
                .iter()
                .map(From::from)
                .collect(),
        }
    }
}

impl From<&aws_sdk_cloudformation::model::StackEvent> for StackEvent {
    fn from(e: &aws_sdk_cloudformation::model::StackEvent) -> Self {
        Self {
            timestamp: e.timestamp.unwrap().fmt(Format::DateTime).unwrap(),
            logical_resource_id: e.logical_resource_id.clone(),
            resource_status: e.resource_status.as_ref().map(|s| s.as_str().to_owned()),
            stack_name: e.stack_name.as_ref().unwrap().clone(),
            resource_status_reason: e.resource_status_reason.clone(),
        }
    }
}

impl From<aws_sdk_cloudformation::output::DescribeStacksOutput> for DescribeStacksOutput {
    fn from(o: aws_sdk_cloudformation::output::DescribeStacksOutput) -> Self {
        Self {
            stacks: o
                .stacks
                .unwrap_or_default()
                .iter()
                .map(From::from)
                .collect(),
        }
    }
}

impl From<&aws_sdk_cloudformation::model::Stack> for Stack {
    fn from(s: &aws_sdk_cloudformation::model::Stack) -> Self {
        Self {
            outputs: s
                .outputs
                .as_ref()
                .map(|o| o.iter().map(From::from).collect()),
        }
    }
}

impl From<&aws_sdk_cloudformation::model::Output> for Output {
    fn from(o: &aws_sdk_cloudformation::model::Output) -> Self {
        Self {
            key: o.output_key.as_ref().unwrap().to_string(),
            value: o.output_value.as_ref().unwrap().to_string(),
        }
    }
}

impl From<aws_sdk_cloudformation::output::DescribeStackResourcesOutput>
    for DescribeStackResourcesOutput
{
    fn from(o: aws_sdk_cloudformation::output::DescribeStackResourcesOutput) -> Self {
        Self {
            stack_resources: o
                .stack_resources
                .unwrap_or_default()
                .iter()
                .map(From::from)
                .collect(),
        }
    }
}

impl From<&aws_sdk_cloudformation::model::StackResource> for StackResource {
    fn from(r: &aws_sdk_cloudformation::model::StackResource) -> Self {
        Self {
            resource_type: r.resource_type.as_ref().unwrap().to_string(),
            physical_resource_id: r.physical_resource_id.clone(),
            stack_name: r.stack_name.as_ref().unwrap().to_string(),
        }
    }
}

impl From<aws_sdk_cloudformation::types::SdkError<aws_sdk_cloudformation::error::DescribeStackEventsError>>
    for DescribeStackEventsError
{
    fn from(
        e: aws_sdk_cloudformation::types::SdkError<
            aws_sdk_cloudformation::error::DescribeStackEventsError,
        >,
    ) -> Self {
        match e {
            aws_sdk_cloudformation::types::SdkError::ConstructionFailure(_) => {
                DescribeStackEventsError::Unknown("construction failure".to_string())
            }
            aws_sdk_cloudformation::types::SdkError::TimeoutError(_) => DescribeStackEventsError::Timeout,
            aws_sdk_cloudformation::types::SdkError::DispatchFailure(_) => {
                DescribeStackEventsError::Dispatch
            }
            aws_sdk_cloudformation::types::SdkError::ResponseError { .. } => {
                DescribeStackEventsError::Response
            }
            aws_sdk_cloudformation::types::SdkError::ServiceError { .. } => {
                DescribeStackEventsError::Service
            }
        }
    }
}

impl From<aws_sdk_cloudformation::types::SdkError<aws_sdk_cloudformation::error::DescribeStacksError>>
    for DescribeStacksError
{
    fn from(
        e: aws_sdk_cloudformation::types::SdkError<aws_sdk_cloudformation::error::DescribeStacksError>,
    ) -> Self {
        match e {
            aws_sdk_cloudformation::types::SdkError::ConstructionFailure(_) => todo!(),
            aws_sdk_cloudformation::types::SdkError::TimeoutError(_) => todo!(),
            aws_sdk_cloudformation::types::SdkError::DispatchFailure(_) => todo!(),
            aws_sdk_cloudformation::types::SdkError::ResponseError { .. } => todo!(),
            aws_sdk_cloudformation::types::SdkError::ServiceError { .. } => todo!(),
        }
    }
}

impl
    From<
        aws_sdk_cloudformation::types::SdkError<
            aws_sdk_cloudformation::error::DescribeStackResourcesError,
        >,
    > for DescribeStackResourcesError
{
    fn from(
        e: aws_sdk_cloudformation::types::SdkError<
            aws_sdk_cloudformation::error::DescribeStackResourcesError,
        >,
    ) -> Self {
        match e {
            aws_sdk_cloudformation::types::SdkError::ConstructionFailure(_) => todo!(),
            aws_sdk_cloudformation::types::SdkError::TimeoutError(_) => todo!(),
            aws_sdk_cloudformation::types::SdkError::DispatchFailure(_) => todo!(),
            aws_sdk_cloudformation::types::SdkError::ResponseError { .. } => todo!(),
            aws_sdk_cloudformation::types::SdkError::ServiceError { .. } => todo!(),
        }
    }
}
