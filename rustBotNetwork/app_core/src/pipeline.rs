use crate::contracts::ToolError;
use crate::invariants::{ensure_json_pointer, ensure_non_empty_trimmed, ensure_range_usize};
use crate::tools::tool_registry::ToolRegistry;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

/// # NDOC
/// component: `pipeline`
/// purpose: Declarative input value for pipeline step parameters.
/// invariants:
///   - `Literal` values are passed through as-is.
///   - `FromStep` references resolve only from previously completed steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PipelineInputValue {
    Literal(Value),
    FromStep { from_step: String, path: String },
}

/// # NDOC
/// component: `pipeline`
/// purpose: One executable unit in a sequential pipeline.
/// invariants:
///   - `id` must be unique within a pipeline.
///   - `tool` must map to an available registry entry at execution time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub id: String,
    pub tool: String,
    pub input: HashMap<String, PipelineInputValue>,
}

/// # NDOC
/// component: `pipeline`
/// purpose: Full user-defined pipeline configuration.
/// invariants:
///   - `steps` must be non-empty and bounded for stability.
///   - references cannot point to future steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDefinition {
    pub name: String,
    pub campaign_id: Option<String>,
    pub steps: Vec<PipelineStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStepStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStepResult {
    pub step_id: String,
    pub tool: String,
    pub status: PipelineStepStatus,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    pub resolved_input: Value,
    pub output: Option<Value>,
    pub error: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRunResult {
    pub pipeline_name: String,
    pub campaign_id: Option<String>,
    pub started_at: String,
    pub finished_at: String,
    pub succeeded: bool,
    pub steps: Vec<PipelineStepResult>,
}

/// # NDOC
/// component: `pipeline`
/// purpose: Execute a sequential pipeline with stop-on-failure semantics.
/// invariants:
///   - Step order is deterministic (definition order).
///   - A failed step terminates remaining execution.
///   - Returned `steps` are prefix-complete up to failure point.
pub async fn execute_pipeline(definition: PipelineDefinition) -> Result<PipelineRunResult, ToolError> {
    validate_pipeline_definition(&definition)?;

    let run_started = Utc::now();
    let mut step_outputs: HashMap<String, Value> = HashMap::new();
    let mut step_results = Vec::new();
    let mut run_succeeded = true;

    let registry = ToolRegistry::new();

    for step in &definition.steps {
        let step_started = Utc::now();
        let resolved_input = match resolve_step_input(&step.input, &step_outputs) {
            Ok(v) => v,
            Err(err) => {
                run_succeeded = false;
                step_results.push(PipelineStepResult {
                    step_id: step.id.clone(),
                    tool: step.tool.clone(),
                    status: PipelineStepStatus::Failed,
                    started_at: step_started.to_rfc3339(),
                    finished_at: Utc::now().to_rfc3339(),
                    duration_ms: (Utc::now() - step_started).num_milliseconds().max(0) as u64,
                    resolved_input: Value::Object(Map::new()),
                    output: None,
                    error: Some(serde_json::json!({
                        "kind": "validation_error",
                        "message": err.message,
                        "retryable": err.retryable,
                        "details": err.details,
                    })),
                });
                break;
            }
        };

        let Some(tool) = registry.get_tool_instance(&step.tool) else {
            run_succeeded = false;
            step_results.push(PipelineStepResult {
                step_id: step.id.clone(),
                tool: step.tool.clone(),
                status: PipelineStepStatus::Failed,
                started_at: step_started.to_rfc3339(),
                finished_at: Utc::now().to_rfc3339(),
                duration_ms: (Utc::now() - step_started).num_milliseconds().max(0) as u64,
                resolved_input,
                output: None,
                error: Some(serde_json::json!({
                    "kind": "validation_error",
                    "message": format!("Unknown or unavailable tool '{}'", step.tool),
                    "retryable": false
                })),
            });
            break;
        };

        match tool.run(resolved_input.clone()).await {
            Ok(output) => {
                step_outputs.insert(step.id.clone(), output.clone());
                step_results.push(PipelineStepResult {
                    step_id: step.id.clone(),
                    tool: step.tool.clone(),
                    status: PipelineStepStatus::Succeeded,
                    started_at: step_started.to_rfc3339(),
                    finished_at: Utc::now().to_rfc3339(),
                    duration_ms: (Utc::now() - step_started).num_milliseconds().max(0) as u64,
                    resolved_input,
                    output: Some(output),
                    error: None,
                });
            }
            Err(err) => {
                run_succeeded = false;
                step_results.push(PipelineStepResult {
                    step_id: step.id.clone(),
                    tool: step.tool.clone(),
                    status: PipelineStepStatus::Failed,
                    started_at: step_started.to_rfc3339(),
                    finished_at: Utc::now().to_rfc3339(),
                    duration_ms: (Utc::now() - step_started).num_milliseconds().max(0) as u64,
                    resolved_input,
                    output: None,
                    error: Some(serde_json::json!({
                        "kind": "tool_execution_error",
                        "message": err.to_string(),
                        "retryable": false
                    })),
                });
                break;
            }
        }
    }

    Ok(PipelineRunResult {
        pipeline_name: definition.name,
        campaign_id: definition.campaign_id,
        started_at: run_started.to_rfc3339(),
        finished_at: Utc::now().to_rfc3339(),
        succeeded: run_succeeded,
        steps: step_results,
    })
}

fn validate_pipeline_definition(definition: &PipelineDefinition) -> Result<(), ToolError> {
    ensure_non_empty_trimmed(&definition.name, "name")?;
    if definition.steps.is_empty() {
        return Err(ToolError::validation("pipeline must include at least one step"));
    }
    ensure_range_usize(definition.steps.len(), 1, 50, "steps.len")?;

    let mut ids = HashSet::new();
    for (idx, step) in definition.steps.iter().enumerate() {
        ensure_non_empty_trimmed(&step.id, "step.id")?;
        if !ids.insert(step.id.clone()) {
            return Err(ToolError::validation(format!(
                "duplicate pipeline step id '{}'",
                step.id
            )));
        }
        ensure_non_empty_trimmed(&step.tool, "step.tool")?;

        for (key, value) in &step.input {
            ensure_non_empty_trimmed(key, "step.input key")?;
            if let PipelineInputValue::FromStep { from_step, path } = value {
                ensure_non_empty_trimmed(from_step, "from_step")?;
                ensure_json_pointer(path, "path")?;
                let prior = &definition.steps[..idx];
                let known = prior.iter().any(|s| s.id == *from_step);
                if !known {
                    return Err(ToolError::validation(format!(
                        "step '{}' references '{}' before it exists",
                        step.id, from_step
                    )));
                }
            }
        }
    }
    Ok(())
}

fn resolve_step_input(
    input: &HashMap<String, PipelineInputValue>,
    outputs: &HashMap<String, Value>,
) -> Result<Value, ToolError> {
    let mut map = Map::new();
    for (key, value) in input {
        let resolved = match value {
            PipelineInputValue::Literal(v) => v.clone(),
            PipelineInputValue::FromStep { from_step, path } => {
                ensure_json_pointer(path, key)?;
                let source = outputs.get(from_step).ok_or_else(|| {
                    ToolError::validation(format!(
                        "input '{}' references missing step '{}'",
                        key, from_step
                    ))
                })?;
                source.pointer(path).cloned().ok_or_else(|| {
                    ToolError::validation(format!(
                        "input '{}' could not resolve path '{}' from step '{}'",
                        key, path, from_step
                    ))
                })?
            }
        };
        map.insert(key.clone(), resolved);
    }
    Ok(Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_step_ids() {
        let definition = PipelineDefinition {
            name: "dup-ids".to_string(),
            campaign_id: None,
            steps: vec![
                PipelineStep {
                    id: "step1".to_string(),
                    tool: "seo_analyzer".to_string(),
                    input: HashMap::new(),
                },
                PipelineStep {
                    id: "step1".to_string(),
                    tool: "competitive_analysis".to_string(),
                    input: HashMap::new(),
                },
            ],
        };

        let err = validate_pipeline_definition(&definition).expect_err("must reject duplicates");
        assert!(err.message.contains("duplicate pipeline step id"));
    }

    #[test]
    fn resolves_input_from_previous_step_output() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "analysis".to_string(),
            serde_json::json!({
                "signal_report_markdown": "report md",
                "source_count": 7
            }),
        );

        let mut input = HashMap::new();
        input.insert(
            "text".to_string(),
            PipelineInputValue::FromStep {
                from_step: "analysis".to_string(),
                path: "/signal_report_markdown".to_string(),
            },
        );

        let resolved = resolve_step_input(&input, &outputs).expect("resolve should succeed");
        assert_eq!(resolved["text"], "report md");
    }

    #[test]
    fn fails_when_reference_path_missing() {
        let mut outputs = HashMap::new();
        outputs.insert("analysis".to_string(), serde_json::json!({"source_count": 7}));

        let mut input = HashMap::new();
        input.insert(
            "text".to_string(),
            PipelineInputValue::FromStep {
                from_step: "analysis".to_string(),
                path: "/signal_report_markdown".to_string(),
            },
        );

        let err = resolve_step_input(&input, &outputs).expect_err("must fail");
        assert!(err.message.contains("could not resolve path"));
    }

    #[test]
    fn rejects_forward_reference() {
        let mut second_input = HashMap::new();
        second_input.insert(
            "text".to_string(),
            PipelineInputValue::FromStep {
                from_step: "third".to_string(),
                path: "/foo".to_string(),
            },
        );

        let definition = PipelineDefinition {
            name: "forward-ref".to_string(),
            campaign_id: None,
            steps: vec![
                PipelineStep {
                    id: "first".to_string(),
                    tool: "seo_analyzer".to_string(),
                    input: HashMap::new(),
                },
                PipelineStep {
                    id: "second".to_string(),
                    tool: "seo_analyzer".to_string(),
                    input: second_input,
                },
                PipelineStep {
                    id: "third".to_string(),
                    tool: "seo_analyzer".to_string(),
                    input: HashMap::new(),
                },
            ],
        };

        let err = validate_pipeline_definition(&definition).expect_err("must fail");
        assert!(err.message.contains("before it exists"));
    }
}
