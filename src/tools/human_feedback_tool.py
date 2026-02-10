from typing import Dict, Any, Optional

# Mock MemoryRetrievalTool for demonstration purposes
class MockMemoryRetrievalTool:
    def store_memory(self, document: Dict[str, Any], metadata: Dict[str, Any]) -> Dict[str, Any]:
        print(f"MockMemoryRetrievalTool.store_memory called with document: {document}, metadata: {metadata}")
        return {"status": "success", "message": "Memory stored (mock)."}

    def retrieve_memory(self, query: str, top_k: int = 1) -> Dict[str, Any]:
        print(f"MockMemoryRetrievalTool.retrieve_memory called with query: {query}, top_k: {top_k}")
        # Simulate some retrieved feedback
        if "positive" in query.lower():
            return {"status": "success", "results": [{"document": {"feedback": "Great work!"}, "metadata": {}}]}
        elif "negative" in query.lower():
            return {"status": "success", "results": [{"document": {"feedback": "Needs improvement."}, "metadata": {}}]}
        return {"status": "success", "results": []}

class HumanFeedbackTool:
    def __init__(self, memory_retrieval_tool: MockMemoryRetrievalTool):
        self.memory_retrieval_tool = memory_retrieval_tool

    def is_available(self) -> bool:
        """This tool is always conceptually available."""
        return True

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Collects or retrieves human feedback.
        Input: { "action": "collect"|"retrieve", "feedback": {...}|"query", "metadata": {...} }
        Output: { "status": "success", ... } or { "status": "error", ... }
        """
        action = input_data.get("action")
        
        if action == "collect":
            feedback_document = input_data.get("feedback")
            metadata = input_data.get("metadata", {})
            if not feedback_document:
                return {"status": "error", "message": "Feedback document is required for 'collect' action."}
            
            return self.memory_retrieval_tool.store_memory(feedback_document, metadata)
        
        elif action == "retrieve":
            query = input_data.get("query")
            top_k = input_data.get("top_k", 1)
            if not query:
                return {"status": "error", "message": "Query is required for 'retrieve' action."}
            
            retrieved_data = self.memory_retrieval_tool.retrieve_memory(query, top_k)
            return retrieved_data
        
        else:
            return {"status": "error", "message": "Invalid action. Must be 'collect' or 'retrieve'."}

# Example Usage
if __name__ == "__main__":
    mock_memory_tool = MockMemoryRetrievalTool()
    tool = HumanFeedbackTool(mock_memory_tool)

    print("--- Testing collect feedback ---")
    collect_result = tool.run({
        "action": "collect",
        "feedback": {"type": "positive", "content": "The campaign was very engaging."}, 
        "metadata": {"campaign_id": "camp123"}
    })
    print(f"Collect result: {collect_result}")

    print("\n--- Testing retrieve positive feedback ---")
    retrieve_positive_result = tool.run({
        "action": "retrieve",
        "query": "positive feedback",
        "top_k": 1
    })
    print(f"Retrieve positive result: {retrieve_positive_result}")

    print("\n--- Testing retrieve negative feedback ---")
    retrieve_negative_result = tool.run({
        "action": "retrieve",
        "query": "negative comments",
        "top_k": 1
    })
    print(f"Retrieve negative result: {retrieve_negative_result}")

    print("\n--- Testing invalid action ---")
    invalid_action_result = tool.run({"action": "unknown"})
    print(f"Invalid action result: {invalid_action_result}")
