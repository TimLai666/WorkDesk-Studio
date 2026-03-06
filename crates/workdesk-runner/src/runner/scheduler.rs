use anyhow::{anyhow, Result};
use std::collections::{HashMap, VecDeque};
use workdesk_core::{WorkflowDefinition, WorkflowNode};

pub fn topological_nodes(workflow: &WorkflowDefinition) -> Result<Vec<WorkflowNode>> {
    let mut node_map = HashMap::<String, WorkflowNode>::new();
    let mut indegree = HashMap::<String, usize>::new();
    let mut adjacency = HashMap::<String, Vec<String>>::new();
    for node in &workflow.nodes {
        node_map.insert(node.id.clone(), node.clone());
        indegree.insert(node.id.clone(), 0);
    }

    for edge in &workflow.edges {
        if !indegree.contains_key(&edge.from) || !indegree.contains_key(&edge.to) {
            return Err(anyhow!(
                "workflow edge references unknown node: {} -> {}",
                edge.from,
                edge.to
            ));
        }
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        *indegree.get_mut(&edge.to).expect("edge target") += 1;
    }

    let mut queue = indegree
        .iter()
        .filter_map(|(node, deg)| (*deg == 0).then_some(node.clone()))
        .collect::<VecDeque<_>>();
    let mut ordered = Vec::with_capacity(workflow.nodes.len());
    while let Some(node_id) = queue.pop_front() {
        if let Some(node) = node_map.get(&node_id) {
            ordered.push(node.clone());
        }
        if let Some(children) = adjacency.get(&node_id) {
            for child in children {
                if let Some(value) = indegree.get_mut(child) {
                    *value -= 1;
                    if *value == 0 {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    }

    if ordered.len() != workflow.nodes.len() {
        return Err(anyhow!("workflow graph contains cycle"));
    }
    Ok(ordered)
}
