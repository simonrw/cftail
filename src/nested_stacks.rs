use crate::aws::{AwsCloudFormationClient, DescribeStackResourcesInput, StackResource};
use eyre::{Result, WrapErr};
use std::collections::HashSet;

pub(crate) async fn fetch_nested_stack_names(
    client: &impl AwsCloudFormationClient,
    root_stack_name: impl Into<String>,
) -> Result<HashSet<String>> {
    let root_stack_name = root_stack_name.into();
    let resources = fetch_stack_resources(client, &root_stack_name)
        .await
        .wrap_err("fetching stack resources")?;

    let mut to_fetch = Vec::new();
    let mut stacks = HashSet::new();
    stacks.insert(root_stack_name.clone());

    let target_resource = String::from("AWS::CloudFormation::Stack");
    for resource in resources {
        if resource.resource_type == target_resource {
            to_fetch.push(resource.physical_resource_id.unwrap());
        }
    }

    while !to_fetch.is_empty() {
        let resource_name = to_fetch.pop();
        if let Some(resource_name) = resource_name {
            let resources = fetch_stack_resources(client, resource_name)
                .await
                .wrap_err("fetching stack resources")?;
            for resource in resources {
                stacks.insert(resource.stack_name);
                if resource.resource_type == target_resource {
                    to_fetch.push(resource.physical_resource_id.unwrap());
                }
            }
        }
    }

    Ok(stacks)
}

pub(crate) async fn fetch_stack_resources(
    client: &impl AwsCloudFormationClient,
    name: impl Into<String>,
) -> Result<Vec<StackResource>> {
    let name = name.into();
    tracing::debug!(name = ?name, "fetching nested resources");
    let res = client
        .describe_stack_resources(DescribeStackResourcesInput { stack_name: name })
        .await
        .unwrap();
    Ok(res.stack_resources)
}
