use eyre::{Context, Result};
use log::debug;
use rusoto_cloudformation::{CloudFormation, CloudFormationClient, DescribeStackResourcesInput};
use rusoto_core::Region;
use std::collections::HashSet;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    root_stack_name: String,
}

async fn fetch_nested_stack_names(
    client: &impl CloudFormation,
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
                stacks.insert(resource.stack_name.unwrap());
                if resource.resource_type == target_resource {
                    to_fetch.push(resource.physical_resource_id.unwrap());
                }
            }
        }
    }

    Ok(stacks)
}

async fn fetch_stack_resources(
    client: &impl CloudFormation,
    name: impl Into<String>,
) -> Result<Vec<rusoto_cloudformation::StackResource>> {
    let name = name.into();
    debug!("fetching resources for {}", name);
    let res = client
        .describe_stack_resources(DescribeStackResourcesInput {
            stack_name: Some(name),
            ..Default::default()
        })
        .await
        .unwrap();
    res.stack_resources
        .ok_or(eyre::eyre!("no stack resources found"))
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let opts = Opts::from_args();

    let region = Region::default();
    let client = CloudFormationClient::new(region);

    let stacks = fetch_nested_stack_names(&client, opts.root_stack_name)
        .await
        .unwrap();

    println!("{:?}, {}", stacks, stacks.len());
}
