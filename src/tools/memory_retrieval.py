from typing import Dict, Any, List, Optional

# Mock ChromaDB client for demonstration purposes
class MockChromaClient:
    def __init__(self):
        self.collections: Dict[str, 'MockChromaCollection'] = {}

    def get_or_create_collection(self, name: str) -> 'MockChromaCollection':
        if name not in self.collections:
            self.collections[name] = MockChromaCollection(name)
        return self.collections[name]

class MockChromaCollection:
    def __init__(self, name: str):
        self.name = name
        self.documents: List[Dict[str, Any]] = []
        self.metadatas: List[Dict[str, Any]] = []
        self.ids: List[str] = []

    def add(self, documents: List[str], metadatas: List[Dict[str, Any]], ids: List[str]):
        print(f"MockChromaCollection '{self.name}'.add called: ids={ids}")
        for doc, meta, id_val in zip(documents, metadatas, ids):
            self.documents.append({"document": doc}) # Store as {"document": "..."}
            self.metadatas.append(meta)
            self.ids.append(id_val)

    def query(self, query_texts: List[str], n_results: int = 1) -> Dict[str, Any]:
        print(f"MockChromaCollection '{self.name}'.query called: query_texts={query_texts}")
        # Very simplified mock query: just return the first n_results documents
        # that contain any of the query_texts (case-insensitive)
        results = []
        for i, doc_item in enumerate(self.documents):
            doc = doc_item["document"]
            if any(q.lower() in doc.lower() for q in query_texts):
                results.append({
                    "id": self.ids[i],
                    "document": doc,
                    "metadata": self.metadatas[i],
                    "distance": 0.1 # Mock distance
                })
            if len(results) >= n_results:
                break
        return {"ids": [r["id"] for r in results], "documents": [r["document"] for r in results], "metadatas": [r["metadata"] for r in results], "distances": [r["distance"] for r in results]}


class MemoryRetrievalTool:
    def __init__(self, chromadb_client: MockChromaClient, collection_name: str = "default_memory_collection"):
        self.client = chromadb_client
        self.collection = self.client.get_or_create_collection(name=collection_name)

    def is_available(self) -> bool:
        """This tool is always conceptually available."""
        return True

    def store_memory(self, document: Dict[str, Any], metadata: Dict[str, Any]) -> Dict[str, Any]:
        """Stores a document (memory) with associated metadata."""
        # Assume document is already a dict, convert to JSON string for ChromaDB storage
        doc_str = json.dumps(document)
        # Generate a simple ID (e.g., from timestamp or hash)
        import hashlib
        id_val = hashlib.md5(doc_str.encode()).hexdigest()

        self.collection.add(
            documents=[doc_str],
            metadatas=[metadata],
            ids=[id_val]
        )
        return {"status": "success", "message": "Memory stored.", "id": id_val}

    def retrieve_memory(self, query: str, n_results: int = 1) -> Dict[str, Any]:
        """Retrieves relevant memories based on a query."""
        results = self.collection.query(
            query_texts=[query],
            n_results=n_results
        )
        # Parse documents back from JSON string if needed
        parsed_results = []
        if "documents" in results and results["documents"]:
            for doc_str, meta in zip(results["documents"], results["metadatas"]):
                try:
                    parsed_results.append({
                        "document": json.loads(doc_str),
                        "metadata": meta
                    })
                except json.JSONDecodeError:
                    parsed_results.append({
                        "document": doc_str, # Keep as string if parsing fails
                        "metadata": meta
                    })
        return {"status": "success", "results": parsed_results}

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Runs the memory retrieval tool.
        Input: { "action": "store"|"retrieve", "document": {...}|"query", "metadata": {...}, "n_results": 1 }
        Output: { "status": "success", ... } or { "status": "error", ... }
        """
        action = input_data.get("action")

        if action == "store":
            document = input_data.get("document")
            metadata = input_data.get("metadata", {})
            if not document:
                return {"status": "error", "message": "Document is required for 'store' action."}
            return self.store_memory(document, metadata)

        elif action == "retrieve":
            query = input_data.get("query")
            n_results = input_data.get("n_results", 1)
            if not query:
                return {"status": "error", "message": "Query is required for 'retrieve' action."}
            return self.retrieve_memory(query, n_results)

        else:
            return {"status": "error", "message": "Invalid action. Must be 'store' or 'retrieve'."}

# Example Usage
if __name__ == "__main__":
    client = MockChromaClient()
    tool = MemoryRetrievalTool(client)

    print("--- Store Memory ---")
    store_result = tool.run({
        "action": "store",
        "document": {"content": "This is a positive customer feedback."},
        "metadata": {"campaign": "spring_sale", "sentiment": "positive"}
    })
    print(store_result)

    store_result2 = tool.run({
        "action": "store",
        "document": {"content": "The budget review was successful."},
        "metadata": {"department": "finance"}
    })
    print(store_result2)

    print("\n--- Retrieve Memory (positive) ---")
    retrieve_result = tool.run({
        "action": "retrieve",
        "query": "positive feedback",
        "n_results": 1
    })
    print(retrieve_result)

    print("\n--- Retrieve Memory (budget) ---")
    retrieve_result2 = tool.run({
        "action": "retrieve",
        "query": "budget",
        "n_results": 1
    })
    print(retrieve_result2)

    print("\n--- Retrieve all (keywords) ---")
    retrieve_all_result = tool.run({
        "action": "retrieve",
        "query": "review",
        "n_results": 5
    })
    print(retrieve_all_result)

    print("\n--- Invalid action ---")
    invalid_action_result = tool.run({"action": "delete"})
    print(invalid_action_result)
