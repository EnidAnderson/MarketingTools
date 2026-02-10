use super::base_tool::BaseTool;
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;

/// Trait defining the interface for a Memory Retrieval Tool.
/// This allows HumanFeedbackTool to interact with a mocked or real memory tool.
#[async_trait]
pub trait MemoryRetrievalToolTrait: Send + Sync {
    async fn store_memory(&self, document: Value, metadata: Value) -> Value;
    async fn retrieve_memory(&self, query: String, top_k: u32) -> Value;
}

/// A Human Feedback Tool that collects and retrieves feedback using a memory retrieval system.
pub struct HumanFeedbackTool {
    memory_retrieval_tool: Box<dyn MemoryRetrievalToolTrait>,
}

impl HumanFeedbackTool {
    /// Creates a new `HumanFeedbackTool` with a specific memory retrieval tool.
    pub fn new(memory_retrieval_tool: Box<dyn MemoryRetrievalToolTrait>) -> Self {
        HumanFeedbackTool {
            memory_retrieval_tool,
        }
    }
}

#[async_trait]
impl BaseTool for HumanFeedbackTool {
    fn name(&self) -> &'static str {
        "HumanFeedbackTool"
    }

    fn description(&self) -> &'static str {
        "Collects or retrieves human feedback. Input requires an 'action' (collect|retrieve). 'collect' needs 'feedback' (JSON) and optional 'metadata' (JSON). 'retrieve' needs 'query' (string) and optional 'top_k' (number)."
    }

    fn is_available(&self) -> bool {
        true // This tool is always conceptually available. Its dependency's availability is external.
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let action = input["action"].as_str();

        match action {
            Some("collect") => {
                let feedback_document = input["feedback"].clone();
                let metadata = input["metadata"].clone();

                if feedback_document.is_null() {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "Feedback document is required for 'collect' action."
                    }));
                }

                Ok(self
                    .memory_retrieval_tool
                    .store_memory(feedback_document, metadata)
                    .await)
            }
            Some("retrieve") => {
                let query = input["query"].as_str();
                let top_k = input["top_k"].as_u64().unwrap_or(1) as u32;

                if query.is_none() {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "Query is required for 'retrieve' action."
                    }));
                }

                Ok(self
                    .memory_retrieval_tool
                    .retrieve_memory(query.unwrap().to_string(), top_k)
                    .await)
            }
            _ => Ok(serde_json::json!({
                "status": "error",
                "message": "Invalid action. Must be 'collect' or 'retrieve'."
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex}; // Added Rc for shared ownership

    // Mock MemoryRetrievalTool implementation
    // This now directly holds the state for verification without needing to clone the Box.
    struct MockMemoryTool {
        pub stored_documents: Mutex<Vec<(Value, Value)>>,
        pub retrieved_query: Mutex<Option<(String, u32)>>,
        pub retrieve_return_value: Mutex<Value>, // Use Mutex for mutable access within retrieve_memory
    }

    impl MockMemoryTool {
        fn new() -> Self {
            MockMemoryTool {
                stored_documents: Mutex::new(Vec::new()),
                retrieved_query: Mutex::new(None),
                retrieve_return_value: Mutex::new(json!({"status": "success", "results": []})),
            }
        }
    }

    // Implement the trait for MockMemoryTool directly
    #[async_trait]
    impl MemoryRetrievalToolTrait for MockMemoryTool {
        async fn store_memory(&self, document: Value, metadata: Value) -> Value {
            self.stored_documents
                .lock()
                .unwrap()
                .push((document, metadata));
            json!({"status": "success", "message": "Memory stored (mock)."})
        }

        async fn retrieve_memory(&self, query: String, top_k: u32) -> Value {
            *self.retrieved_query.lock().unwrap() = Some((query, top_k));
            self.retrieve_return_value.lock().unwrap().clone()
        }
    }

    // Implement the trait for Arc<MockMemoryTool> so it can be used as a trait object
    // This delegates the trait calls to the inner MockMemoryTool
    #[async_trait]
    impl MemoryRetrievalToolTrait for Arc<MockMemoryTool> {
        async fn store_memory(&self, document: Value, metadata: Value) -> Value {
            self.as_ref().store_memory(document, metadata).await
        }

        async fn retrieve_memory(&self, query: String, top_k: u32) -> Value {
            self.as_ref().retrieve_memory(query, top_k).await
        }
    }

    // Helper function to create a tool and get a reference to its mock
    fn setup_human_feedback_tool() -> (HumanFeedbackTool, Arc<MockMemoryTool>) {
        let mock_memory = Arc::new(MockMemoryTool::new());
        // Now, Box::new takes an Arc<MockMemoryTool> which implements the trait
        let tool = HumanFeedbackTool::new(Box::new(mock_memory.clone()));
        (tool, mock_memory)
    }

    #[test]
    fn test_human_feedback_tool_name() {
        let (tool, _mock_memory) = setup_human_feedback_tool();
        assert_eq!(tool.name(), "HumanFeedbackTool");
    }

    #[test]
    fn test_human_feedback_tool_is_available() {
        let (tool, _mock_memory) = setup_human_feedback_tool();
        assert!(tool.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_collect_feedback_success() {
        // Added async
        let (tool, mock_memory) = setup_human_feedback_tool();

        let feedback_doc = json!({"type": "positive", "content": "Great work!"});
        let metadata_doc = json!({"campaign_id": "camp123"});

        let input = json!({
            "action": "collect",
            "feedback": feedback_doc,
            "metadata": metadata_doc
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        // Access stored_documents via the original mock_memory instance
        let stored_docs = mock_memory.stored_documents.lock().unwrap();
        assert_eq!(stored_docs.len(), 1);
        assert_eq!(stored_docs[0].0, feedback_doc);
        assert_eq!(stored_docs[0].1, metadata_doc);
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_collect_feedback_missing_feedback_document() {
        // Added async
        let (tool, mock_memory) = setup_human_feedback_tool();

        let input = json!({
            "action": "collect",
            "metadata": {"campaign_id": "camp123"}
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Feedback document is required"));
        let stored_docs = mock_memory.stored_documents.lock().unwrap();
        assert!(stored_docs.is_empty());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_retrieve_feedback_success() {
        // Added async
        let (tool, mock_memory) = setup_human_feedback_tool();

        let expected_query = "positive feedback".to_string();
        let expected_top_k = 2;
        // Set the expected return value for this specific mock instance
        *mock_memory.retrieve_return_value.lock().unwrap() = json!({
            "status": "success",
            "results": [
                {"document": {"feedback": "Positive review."}, "metadata": {}},
                {"document": {"feedback": "Good job."}, "metadata": {}}
            ]
        });

        let input = json!({
            "action": "retrieve",
            "query": expected_query.clone(), // Clone for input
            "top_k": expected_top_k
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        // Verify the query and top_k passed to the mock
        assert_eq!(
            *mock_memory.retrieved_query.lock().unwrap(),
            Some((expected_query.clone(), expected_top_k))
        );
        // Verify the returned result matches the mock's configured return value
        assert_eq!(result, *mock_memory.retrieve_return_value.lock().unwrap());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_retrieve_feedback_missing_query() {
        // Added async
        let (tool, mock_memory) = setup_human_feedback_tool();

        let input = json!({
            "action": "retrieve",
            "top_k": 5
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Query is required"));
        assert!(mock_memory.retrieved_query.lock().unwrap().is_none());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_invalid_action() {
        // Added async
        let (tool, _mock_memory) = setup_human_feedback_tool();

        let input = json!({"action": "unknown_action"});
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Invalid action"));
    }
}
