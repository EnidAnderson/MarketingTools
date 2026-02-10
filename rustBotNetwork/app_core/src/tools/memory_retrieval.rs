use super::base_tool::BaseTool;
#[allow(unused_imports)]
// Used because it's implemented for MemoryRetrievalTool in this module
use super::human_feedback_tool::MemoryRetrievalToolTrait;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
// Removed: use serde_json::json; // Added json macro import
use async_trait::async_trait;
use itertools::izip; // Used by MockChromaCollection::add

// --- ChromaDB Mock Traits ---

/// Trait defining the interface for a ChromaDB collection.
pub trait ChromaCollectionTrait {
    fn name(&self) -> &str;
    fn add(&self, documents: Vec<String>, metadatas: Vec<Value>, ids: Vec<String>);
    fn query(&self, query_texts: Vec<String>, n_results: u32) -> Value;
}

/// Mock implementation of a ChromaDB collection.
pub struct MockChromaCollection {
    pub name: String,
    pub documents: Mutex<Vec<String>>,
    pub metadatas: Mutex<Vec<Value>>,
    pub ids: Mutex<Vec<String>>,
}

impl MockChromaCollection {
    pub fn new(name: String) -> Self {
        MockChromaCollection {
            name,
            documents: Mutex::new(Vec::new()),
            metadatas: Mutex::new(Vec::new()),
            ids: Mutex::new(Vec::new()),
        }
    }
}

impl ChromaCollectionTrait for MockChromaCollection {
    fn name(&self) -> &str {
        &self.name
    }

    fn add(&self, documents: Vec<String>, metadatas: Vec<Value>, ids: Vec<String>) {
        let mut docs_borrow = self.documents.lock().unwrap();
        let mut metas_borrow = self.metadatas.lock().unwrap();
        let mut ids_borrow = self.ids.lock().unwrap();

        for (doc, meta, id_val) in izip!(documents, metadatas, ids) {
            docs_borrow.push(doc); // Store String directly
            metas_borrow.push(meta);
            ids_borrow.push(id_val);
        }
    }

    fn query(&self, query_texts: Vec<String>, n_results: u32) -> Value {
        let mut results = Vec::new();
        let docs_borrow = self.documents.lock().unwrap();
        let metas_borrow = self.metadatas.lock().unwrap();
        let ids_borrow = self.ids.lock().unwrap();

        for (i, doc_str) in docs_borrow.iter().enumerate() {
            // doc_str is now &String
            if query_texts
                .iter()
                .any(|q| doc_str.to_lowercase().contains(&q.to_lowercase()))
            {
                results.push(serde_json::json!({
                    "id": ids_borrow[i],
                    "document": serde_json::from_str::<Value>(doc_str).unwrap(), // Parse doc_str into Value here
                    "metadata": metas_borrow[i],
                    "distance": 0.1 // Mock distance
                }));
            }
            if results.len() as u32 >= n_results {
                break;
            }
        }

        // This structure maps to Python's results = {"ids": [...], "documents": [...], "metadatas": [...], "distances": [...]}
        // where 'documents' is a list of the actual document values.
        serde_json::json!({
            "ids": results.iter().map(|r| r["id"].clone()).collect::<Vec<Value>>(),
            "documents": results.iter().map(|r| r["document"].clone()).collect::<Vec<Value>>(),
            "metadatas": results.iter().map(|r| r["metadata"].clone()).collect::<Vec<Value>>(),
            "distances": results.iter().map(|r| r["distance"].clone()).collect::<Vec<Value>>(),
        })
    }
}

/// Trait defining the interface for a ChromaDB client.
pub trait ChromaClientTrait: Send + Sync {
    fn get_or_create_collection(&self, name: &str) -> Arc<dyn ChromaCollectionTrait>;
}

/// Mock implementation of a ChromaDB client.
pub struct MockChromaClient {
    pub collections: Mutex<HashMap<String, Arc<MockChromaCollection>>>,
}

impl MockChromaClient {
    pub fn new() -> Self {
        MockChromaClient {
            collections: Mutex::new(HashMap::new()),
        }
    }
}

impl ChromaClientTrait for MockChromaClient {
    fn get_or_create_collection(&self, name: &str) -> Arc<dyn ChromaCollectionTrait> {
        let mut collections_borrow = self.collections.lock().unwrap();
        if !collections_borrow.contains_key(name) {
            collections_borrow.insert(
                name.to_string(),
                Arc::new(MockChromaCollection::new(name.to_string())),
            );
        }
        collections_borrow.get(name).unwrap().clone()
    }
}

// Implement ChromaClientTrait for Arc<MockChromaClient> so it can be used as a trait object
impl ChromaClientTrait for Arc<MockChromaClient> {
    fn get_or_create_collection(&self, name: &str) -> Arc<dyn ChromaCollectionTrait> {
        self.as_ref().get_or_create_collection(name)
    }
}

// Implement ChromaCollectionTrait for Arc<MockChromaCollection> to allow it to be used as a trait object
impl ChromaCollectionTrait for Arc<MockChromaCollection> {
    fn name(&self) -> &str {
        self.as_ref().name()
    }
    fn add(&self, documents: Vec<String>, metadatas: Vec<Value>, ids: Vec<String>) {
        self.as_ref().add(documents, metadatas, ids)
    }
    fn query(&self, query_texts: Vec<String>, n_results: u32) -> Value {
        self.as_ref().query(query_texts, n_results)
    }
}

// --- MemoryRetrievalTool ---

pub struct MemoryRetrievalTool {
    client: Box<dyn ChromaClientTrait>,
    collection_name: String,
}

impl MemoryRetrievalTool {
    pub fn new(client: Box<dyn ChromaClientTrait>, collection_name: String) -> Self {
        MemoryRetrievalTool {
            client,
            collection_name,
        }
    }

    fn get_collection(&self) -> Arc<dyn ChromaCollectionTrait> {
        self.client.get_or_create_collection(&self.collection_name)
    }

    // --- Implement MemoryRetrievalToolTrait ---
    // Note: This MemoryRetrievalTool also implements the trait HumanFeedbackTool expects
    // This allows it to be used as the concrete memory_retrieval_tool for HumanFeedbackTool.
    async fn store_memory_impl(&self, document: Value, metadata: Value) -> Value {
        let collection = self.get_collection();
        // Change to document.to_string() to get raw JSON object string
        let doc_str = document.to_string(); // Corrected
        let id_val = format!("{:x}", Sha256::digest(doc_str.as_bytes())); // Generate ID

        collection.add(vec![doc_str], vec![metadata], vec![id_val.clone()]);
        serde_json::json!({"status": "success", "message": "Memory stored.", "id": id_val})
    }

    async fn retrieve_memory_impl(&self, query: String, n_results: u32) -> Value {
        let collection = self.get_collection();
        let query_results_from_chroma_mock = collection.query(vec![query], n_results);
        dbg!(&query_results_from_chroma_mock); // Debug output

        let parsed_results: Vec<Value> = query_results_from_chroma_mock["documents"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .zip(
                query_results_from_chroma_mock["metadatas"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter(),
            )
            .filter_map(|(doc_val, meta_val)| {
                // doc_val is now directly the parsed document JSON object from MockChromaCollection::query
                let parsed_doc = doc_val.clone();

                Some(serde_json::json!({
                    "document": parsed_doc,
                    "metadata": meta_val.clone(), // Clone meta_val here as well
                }))
            })
            .collect();

        serde_json::json!({"status": "success", "results": parsed_results})
    }
}

// Implement the MemoryRetrievalToolTrait for MemoryRetrievalTool
#[async_trait]
impl MemoryRetrievalToolTrait for MemoryRetrievalTool {
    async fn store_memory(&self, document: Value, metadata: Value) -> Value {
        self.store_memory_impl(document, metadata).await
    }

    async fn retrieve_memory(&self, query: String, top_k: u32) -> Value {
        self.retrieve_memory_impl(query, top_k).await
    }
}

// --- Implement BaseTool for MemoryRetrievalTool ---
#[async_trait]
impl BaseTool for MemoryRetrievalTool {
    fn name(&self) -> &'static str {
        "MemoryRetrievalTool"
    }

    fn description(&self) -> &'static str {
        "Stores and retrieves memories (documents) using a ChromaDB-like system. Actions: 'store' (requires 'document' and 'metadata'), 'retrieve' (requires 'query' and optional 'n_results')."
    }

    fn is_available(&self) -> bool {
        // Conceptually always available as it relies on its client being injected
        true
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let action = input["action"].as_str();

        match action {
            Some("store") => {
                let document = input["document"].clone();
                let metadata = input["metadata"].clone();
                if document.is_null() {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "Document is required for 'store' action."
                    }));
                }
                Ok(self.store_memory_impl(document, metadata).await)
            }
            Some("retrieve") => {
                let query = input["query"].as_str();
                let n_results = input["n_results"].as_u64().unwrap_or(1) as u32;
                if query.is_none() {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "Query is required for 'retrieve' action."
                    }));
                }
                Ok(self
                    .retrieve_memory_impl(query.unwrap().to_string(), n_results)
                    .await)
            }
            _ => Ok(serde_json::json!({
                "status": "error",
                "message": "Invalid action. Must be 'store' or 'retrieve'."
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Helper to create a new MemoryRetrievalTool with a fresh MockChromaClient
    fn setup_memory_retrieval_tool() -> (MemoryRetrievalTool, Arc<MockChromaClient>) {
        let mock_client = Arc::new(MockChromaClient::new());
        let tool =
            MemoryRetrievalTool::new(Box::new(mock_client.clone()), "test_collection".to_string());
        (tool, mock_client)
    }

    #[test]
    fn test_memory_retrieval_tool_name() {
        let (tool, _mock_client) = setup_memory_retrieval_tool();
        assert_eq!(tool.name(), "MemoryRetrievalTool");
    }

    #[test]
    fn test_memory_retrieval_tool_is_available() {
        let (tool, _mock_client) = setup_memory_retrieval_tool();
        assert!(tool.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_store_memory_success() {
        // Added async
        let (tool, mock_client) = setup_memory_retrieval_tool();

        let doc_to_store = json!({"content": "This is a test document."});
        let meta_to_store = json!({"source": "test", "tag": "unit"});

        let input = json!({
            "action": "store",
            "document": doc_to_store,
            "metadata": meta_to_store
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        assert!(result["id"].as_str().is_some());

        // Verify stored in mock client
        let collections = mock_client.collections.lock().unwrap(); // Changed .borrow() to .lock().unwrap()
        let collection = collections.get("test_collection").unwrap();
        let stored_docs = collection.documents.lock().unwrap(); // Changed .borrow() to .lock().unwrap()
        let stored_metas = collection.metadatas.lock().unwrap(); // Changed .borrow() to .lock().unwrap()
        let stored_ids = collection.ids.lock().unwrap(); // Changed .borrow() to .lock().unwrap()

        assert_eq!(stored_docs.len(), 1);
        // Compare the JSON Value representation, not the raw string
        assert_eq!(
            serde_json::from_str::<Value>(&stored_docs[0]).unwrap(),
            doc_to_store
        );
        assert_eq!(stored_metas[0], meta_to_store);
        assert_eq!(stored_ids.len(), 1);
        assert_eq!(stored_ids[0], result["id"]);
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_store_memory_missing_document() {
        // Added async
        let (tool, _mock_client) = setup_memory_retrieval_tool();

        let input = json!({
            "action": "store",
            "metadata": {"source": "test"}
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Document is required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_retrieve_memory_success() {
        // Added async
        let (tool, mock_client) = setup_memory_retrieval_tool();

        {
            // Ensure the collection exists before we try to populate it directly
            tool.get_collection();

            // Populate mock with data first
            let collections = mock_client.collections.lock().unwrap(); // Changed .borrow() to .lock().unwrap()
            let collection = collections.get("test_collection").unwrap();
            collection.add(
                vec![json!({"content": "positive feedback"}).to_string()],
                vec![json!({"sentiment": "positive"})],
                vec!["id1".to_string()],
            );
            collection.add(
                vec![json!({"content": "negative review"}).to_string()],
                vec![json!({"sentiment": "negative"})],
                vec!["id2".to_string()],
            );
        }

        let query = "positive".to_string();
        let n_results = 1;

        let input = json!({
            "action": "retrieve",
            "query": query,
            "n_results": n_results
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        let results_array = result["results"].as_array().unwrap();
        assert_eq!(results_array.len(), 1);
        assert_eq!(results_array[0]["document"]["content"], "positive feedback");
        assert_eq!(results_array[0]["metadata"]["sentiment"], "positive");
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_retrieve_memory_multiple_results() {
        // Added async
        let (tool, mock_client) = setup_memory_retrieval_tool();

        {
            // Ensure the collection exists before we try to populate it directly
            tool.get_collection();

            // Populate mock with data first
            let collections = mock_client.collections.lock().unwrap(); // Changed .borrow() to .lock().unwrap()
            let collection = collections.get("test_collection").unwrap();
            collection.add(
                vec![json!({"content": "positive feedback A"}).to_string()],
                vec![json!({"sentiment": "positive"})],
                vec!["idA".to_string()],
            );
            collection.add(
                vec![json!({"content": "neutral feedback B"}).to_string()],
                vec![json!({"sentiment": "neutral"})],
                vec!["idB".to_string()],
            );
            collection.add(
                vec![json!({"content": "positive feedback C"}).to_string()],
                vec![json!({"sentiment": "positive"})],
                vec!["idC".to_string()],
            );
        }

        let query = "positive".to_string();
        let n_results = 2;

        let input = json!({
            "action": "retrieve",
            "query": query,
            "n_results": n_results
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        let results_array = result["results"].as_array().unwrap();
        assert_eq!(results_array.len(), 2);
        let content_a = json!({"content": "positive feedback A"});
        let content_c = json!({"content": "positive feedback C"});
        assert!(results_array.iter().any(|r| r["document"] == content_a));
        assert!(results_array.iter().any(|r| r["document"] == content_c));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_retrieve_memory_missing_query() {
        // Added async
        let (tool, _mock_client) = setup_memory_retrieval_tool();

        let input = json!({
            "action": "retrieve",
            "n_results": 5
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Query is required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_invalid_action() {
        // Added async
        let (tool, _mock_client) = setup_memory_retrieval_tool();

        let input = json!({"action": "unknown_action"});
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Invalid action"));
    }
}
