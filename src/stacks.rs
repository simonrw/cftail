use crate::aws::AwsCloudFormationClient;
use eyre::{Context, Result};
use std::{collections::HashSet, iter::FromIterator};

#[derive(Debug, Clone)]
pub(crate) struct StackInfo {
    pub(crate) names: HashSet<String>,
    pub(crate) original_names: HashSet<String>,
}

pub(crate) async fn build_stack_list(
    client: &impl AwsCloudFormationClient,
    stacks: &[String],
    nested: bool,
) -> Result<StackInfo> {
    let original_names = HashSet::from_iter(stacks.iter().cloned());
    if nested {
        let mut names = HashSet::new();
        for stack in stacks {
            let nested = crate::nested_stacks::fetch_nested_stack_names(client, stack)
                .await
                .wrap_err("fetching nested stack names")?;
            names.extend(nested.iter().cloned());
        }

        Ok(StackInfo {
            names,
            original_names,
        })
    } else {
        let names = HashSet::from_iter(stacks.iter().cloned());
        Ok(StackInfo {
            names,
            original_names,
        })
    }
}
