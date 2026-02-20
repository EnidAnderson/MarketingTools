use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_GRAPH_NODES: usize = 64;
const MAX_GRAPH_EDGES: usize = 256;

/// # NDOC
/// component: `subsystems::agent_graph`
/// purpose: Canonical node role taxonomy for graph-based agent workflows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentGraphNodeKindV1 {
    Planner,
    Generator,
    ToolCall,
    Critic,
    Refiner,
    ReviewGate,
    Merge,
}

/// # NDOC
/// component: `subsystems::agent_graph`
/// purpose: One graph node declaration with deterministic id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentGraphNodeV1 {
    pub node_id: String,
    pub kind: AgentGraphNodeKindV1,
    pub description: String,
    #[serde(default)]
    pub params: Value,
}

/// # NDOC
/// component: `subsystems::agent_graph`
/// purpose: Conditional edge semantics used by graph runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentGraphEdgeConditionV1 {
    Always,
    OnSuccess,
    OnFailure,
    ScoreAtLeast { metric: String, threshold: f64 },
}

/// # NDOC
/// component: `subsystems::agent_graph`
/// purpose: Directed edge between two graph nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentGraphEdgeV1 {
    pub from_node_id: String,
    pub to_node_id: String,
    pub condition: AgentGraphEdgeConditionV1,
}

/// # NDOC
/// component: `subsystems::agent_graph`
/// purpose: Full typed DAG contract used for graph-driven workflows.
/// invariants:
///   - Graph must be acyclic.
///   - Entry node must exist.
///   - Node ids are unique.
///   - Every node must be reachable from entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentGraphDefinitionV1 {
    pub graph_id: String,
    pub version: String,
    pub entry_node_id: String,
    pub nodes: Vec<AgentGraphNodeV1>,
    pub edges: Vec<AgentGraphEdgeV1>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentGraphValidationError {
    pub code: String,
    pub message: String,
    pub field_paths: Vec<String>,
}

impl AgentGraphValidationError {
    fn new(code: impl Into<String>, message: impl Into<String>, field_paths: Vec<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field_paths,
        }
    }
}

impl std::fmt::Display for AgentGraphValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AgentGraphValidationError {}

/// # NDOC
/// component: `subsystems::agent_graph`
/// purpose: Validate graph contract before any runtime execution.
pub fn validate_agent_graph_definition_v1(
    graph: &AgentGraphDefinitionV1,
) -> Result<(), AgentGraphValidationError> {
    if graph.graph_id.trim().is_empty() {
        return Err(AgentGraphValidationError::new(
            "graph_id_required",
            "graph_id cannot be empty",
            vec!["graph_id".to_string()],
        ));
    }
    if graph.version.trim().is_empty() {
        return Err(AgentGraphValidationError::new(
            "graph_version_required",
            "version cannot be empty",
            vec!["version".to_string()],
        ));
    }
    if graph.entry_node_id.trim().is_empty() {
        return Err(AgentGraphValidationError::new(
            "entry_node_required",
            "entry_node_id cannot be empty",
            vec!["entry_node_id".to_string()],
        ));
    }
    if graph.nodes.is_empty() {
        return Err(AgentGraphValidationError::new(
            "graph_nodes_required",
            "graph must include at least one node",
            vec!["nodes".to_string()],
        ));
    }
    if graph.nodes.len() > MAX_GRAPH_NODES {
        return Err(AgentGraphValidationError::new(
            "graph_nodes_limit_exceeded",
            format!("graph supports at most {MAX_GRAPH_NODES} nodes"),
            vec!["nodes".to_string()],
        ));
    }
    if graph.edges.len() > MAX_GRAPH_EDGES {
        return Err(AgentGraphValidationError::new(
            "graph_edges_limit_exceeded",
            format!("graph supports at most {MAX_GRAPH_EDGES} edges"),
            vec!["edges".to_string()],
        ));
    }

    let mut node_ids = BTreeSet::new();
    for (idx, node) in graph.nodes.iter().enumerate() {
        if node.node_id.trim().is_empty() {
            return Err(AgentGraphValidationError::new(
                "node_id_required",
                "node_id cannot be empty",
                vec![format!("nodes[{idx}].node_id")],
            ));
        }
        if node.description.trim().is_empty() {
            return Err(AgentGraphValidationError::new(
                "node_description_required",
                "node description cannot be empty",
                vec![format!("nodes[{idx}].description")],
            ));
        }
        if !node_ids.insert(node.node_id.clone()) {
            return Err(AgentGraphValidationError::new(
                "duplicate_node_id",
                format!("duplicate node_id '{}'", node.node_id),
                vec![format!("nodes[{idx}].node_id")],
            ));
        }
    }
    if !node_ids.contains(&graph.entry_node_id) {
        return Err(AgentGraphValidationError::new(
            "entry_node_missing",
            "entry_node_id must reference an existing node",
            vec!["entry_node_id".to_string()],
        ));
    }

    for (idx, edge) in graph.edges.iter().enumerate() {
        if edge.from_node_id.trim().is_empty() {
            return Err(AgentGraphValidationError::new(
                "edge_from_required",
                "edge.from_node_id cannot be empty",
                vec![format!("edges[{idx}].from_node_id")],
            ));
        }
        if edge.to_node_id.trim().is_empty() {
            return Err(AgentGraphValidationError::new(
                "edge_to_required",
                "edge.to_node_id cannot be empty",
                vec![format!("edges[{idx}].to_node_id")],
            ));
        }
        if edge.from_node_id == edge.to_node_id {
            return Err(AgentGraphValidationError::new(
                "edge_self_loop_forbidden",
                "self-loop edges are not allowed",
                vec![format!("edges[{idx}]")],
            ));
        }
        if !node_ids.contains(&edge.from_node_id) {
            return Err(AgentGraphValidationError::new(
                "edge_from_unknown",
                format!("edge.from_node_id '{}' does not exist", edge.from_node_id),
                vec![format!("edges[{idx}].from_node_id")],
            ));
        }
        if !node_ids.contains(&edge.to_node_id) {
            return Err(AgentGraphValidationError::new(
                "edge_to_unknown",
                format!("edge.to_node_id '{}' does not exist", edge.to_node_id),
                vec![format!("edges[{idx}].to_node_id")],
            ));
        }
        if let AgentGraphEdgeConditionV1::ScoreAtLeast { metric, threshold } = &edge.condition {
            if metric.trim().is_empty() {
                return Err(AgentGraphValidationError::new(
                    "edge_metric_required",
                    "score condition metric cannot be empty",
                    vec![format!("edges[{idx}].condition.metric")],
                ));
            }
            if !threshold.is_finite() || *threshold < 0.0 || *threshold > 1.0 {
                return Err(AgentGraphValidationError::new(
                    "edge_threshold_out_of_range",
                    "score threshold must be in range [0.0, 1.0]",
                    vec![format!("edges[{idx}].condition.threshold")],
                ));
            }
        }
    }

    let order = deterministic_topological_order_v1(graph)?;
    let reachable = reachable_from_entry(graph);
    if reachable.len() != graph.nodes.len() {
        for node in &graph.nodes {
            if !reachable.contains(&node.node_id) {
                return Err(AgentGraphValidationError::new(
                    "unreachable_node",
                    format!("node '{}' is unreachable from entry", node.node_id),
                    vec![format!("nodes[{}]", node.node_id)],
                ));
            }
        }
    }
    assert_eq!(
        order.len(),
        graph.nodes.len(),
        "validated graph must have topological order containing every node"
    );
    Ok(())
}

/// # NDOC
/// component: `subsystems::agent_graph`
/// purpose: Return deterministic topological order for DAG execution scheduling.
pub fn deterministic_topological_order_v1(
    graph: &AgentGraphDefinitionV1,
) -> Result<Vec<String>, AgentGraphValidationError> {
    let mut indegree: BTreeMap<String, usize> = BTreeMap::new();
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in &graph.nodes {
        indegree.insert(node.node_id.clone(), 0);
        adjacency.insert(node.node_id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        if let Some(value) = indegree.get_mut(&edge.to_node_id) {
            *value += 1;
        }
        adjacency
            .entry(edge.from_node_id.clone())
            .or_default()
            .push(edge.to_node_id.clone());
    }
    for children in adjacency.values_mut() {
        children.sort();
        children.dedup();
    }

    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter_map(|(node_id, degree)| {
            if *degree == 0 {
                Some(node_id.clone())
            } else {
                None
            }
        })
        .collect();
    let mut order = Vec::with_capacity(graph.nodes.len());
    while let Some(node_id) = queue.pop_front() {
        order.push(node_id.clone());
        if let Some(children) = adjacency.get(&node_id) {
            for child in children {
                if let Some(degree) = indegree.get_mut(child) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    }
    if order.len() != graph.nodes.len() {
        return Err(AgentGraphValidationError::new(
            "graph_cycle_detected",
            "graph contains at least one cycle",
            vec!["edges".to_string()],
        ));
    }
    Ok(order)
}

fn reachable_from_entry(graph: &AgentGraphDefinitionV1) -> BTreeSet<String> {
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in &graph.nodes {
        adjacency.insert(node.node_id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        adjacency
            .entry(edge.from_node_id.clone())
            .or_default()
            .push(edge.to_node_id.clone());
    }
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(graph.entry_node_id.clone());
    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if let Some(children) = adjacency.get(&node_id) {
            for child in children {
                queue.push_back(child.clone());
            }
        }
    }
    visited
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_graph() -> AgentGraphDefinitionV1 {
        AgentGraphDefinitionV1 {
            graph_id: "wf.test.v1".to_string(),
            version: "1".to_string(),
            entry_node_id: "planner".to_string(),
            nodes: vec![
                AgentGraphNodeV1 {
                    node_id: "planner".to_string(),
                    kind: AgentGraphNodeKindV1::Planner,
                    description: "Plan steps".to_string(),
                    params: Value::Null,
                },
                AgentGraphNodeV1 {
                    node_id: "generator".to_string(),
                    kind: AgentGraphNodeKindV1::Generator,
                    description: "Generate draft".to_string(),
                    params: Value::Null,
                },
                AgentGraphNodeV1 {
                    node_id: "critic".to_string(),
                    kind: AgentGraphNodeKindV1::Critic,
                    description: "Critique draft".to_string(),
                    params: Value::Null,
                },
                AgentGraphNodeV1 {
                    node_id: "gate".to_string(),
                    kind: AgentGraphNodeKindV1::ReviewGate,
                    description: "Gate output".to_string(),
                    params: Value::Null,
                },
            ],
            edges: vec![
                AgentGraphEdgeV1 {
                    from_node_id: "planner".to_string(),
                    to_node_id: "generator".to_string(),
                    condition: AgentGraphEdgeConditionV1::Always,
                },
                AgentGraphEdgeV1 {
                    from_node_id: "generator".to_string(),
                    to_node_id: "critic".to_string(),
                    condition: AgentGraphEdgeConditionV1::Always,
                },
                AgentGraphEdgeV1 {
                    from_node_id: "critic".to_string(),
                    to_node_id: "gate".to_string(),
                    condition: AgentGraphEdgeConditionV1::ScoreAtLeast {
                        metric: "instruction_coverage".to_string(),
                        threshold: 0.6,
                    },
                },
            ],
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn valid_graph_passes_validation() {
        let graph = valid_graph();
        assert!(validate_agent_graph_definition_v1(&graph).is_ok());
    }

    #[test]
    fn cycle_is_rejected() {
        let mut graph = valid_graph();
        graph.edges.push(AgentGraphEdgeV1 {
            from_node_id: "gate".to_string(),
            to_node_id: "planner".to_string(),
            condition: AgentGraphEdgeConditionV1::Always,
        });
        let err = validate_agent_graph_definition_v1(&graph).expect_err("must reject cycle");
        assert_eq!(err.code, "graph_cycle_detected");
    }

    #[test]
    fn duplicate_node_ids_are_rejected() {
        let mut graph = valid_graph();
        graph.nodes.push(AgentGraphNodeV1 {
            node_id: "planner".to_string(),
            kind: AgentGraphNodeKindV1::ToolCall,
            description: "duplicate".to_string(),
            params: Value::Null,
        });
        let err = validate_agent_graph_definition_v1(&graph).expect_err("must reject duplicate");
        assert_eq!(err.code, "duplicate_node_id");
    }

    #[test]
    fn unreachable_nodes_are_rejected() {
        let mut graph = valid_graph();
        graph.nodes.push(AgentGraphNodeV1 {
            node_id: "orphan".to_string(),
            kind: AgentGraphNodeKindV1::Generator,
            description: "orphan".to_string(),
            params: Value::Null,
        });
        let err = validate_agent_graph_definition_v1(&graph).expect_err("must reject orphan");
        assert_eq!(err.code, "unreachable_node");
    }

    #[test]
    fn invalid_score_threshold_is_rejected() {
        let mut graph = valid_graph();
        graph.edges[2].condition = AgentGraphEdgeConditionV1::ScoreAtLeast {
            metric: "instruction_coverage".to_string(),
            threshold: 1.25,
        };
        let err = validate_agent_graph_definition_v1(&graph).expect_err("must reject threshold");
        assert_eq!(err.code, "edge_threshold_out_of_range");
    }

    #[test]
    fn topological_order_is_deterministic() {
        let graph = valid_graph();
        let order_a = deterministic_topological_order_v1(&graph).expect("order a");
        let order_b = deterministic_topological_order_v1(&graph).expect("order b");
        assert_eq!(order_a, order_b);
        assert_eq!(order_a[0], "planner");
    }
}
