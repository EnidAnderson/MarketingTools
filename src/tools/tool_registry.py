from typing import Dict, Any, List, Optional
import json

# Minimal BaseTool interface for Python to simulate the Rust trait
class BaseToolPython:
    def name(self) -> str:
        raise NotImplementedError
    
    def description(self) -> str:
        raise NotImplementedError

    def is_available(self) -> bool:
        raise NotImplementedError

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        raise NotImplementedError

class ToolRegistry:
    def __init__(self):
        self._tools: Dict[str, BaseToolPython] = {}

    def register_tool(self, tool: BaseToolPython):
        """Registers a tool with the registry."""
        self._tools[tool.name()] = tool

    def get_tool_instance(self, tool_name: str) -> Optional[BaseToolPython]:
        """
        Returns an instantiated tool if available and its `is_available()` method returns True.
        Returns None if not found or not available.
        """
        tool = self._tools.get(tool_name)
        if tool and tool.is_available():
            return tool
        return None

    def get_available_tool_descriptions(self) -> List[Dict[str, Any]]:
        """
        Returns a list of OpenAPI-like JSON schema descriptions for all available tools.
        (Simplified for this placeholder)
        """
        descriptions = []
        for tool_name, tool_instance in self._tools.items():
            if tool_instance.is_available():
                # This is a highly simplified representation of a tool description
                descriptions.append({
                    "name": tool_instance.name(),
                    "description": tool_instance.description(),
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            # Placeholder: in a real system, this would come from tool metadata
                            "example_param": {"type": "string", "description": "An example parameter"}
                        }
                    }
                })
        return descriptions

# Example Usage (assuming some dummy tools)
if __name__ == "__main__":
    class DummyEmailSender(BaseToolPython):
        def name(self) -> str: return "EmailSender"
        def description(self) -> str: return "Sends emails."
        def is_available(self) -> bool: return True
        def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
            print(f"Dummy EmailSender running with: {input_data}")
            return {"status": "success", "message": "Dummy email sent."}

    class DummyImageManipulator(BaseToolPython):
        def name(self) -> str: return "ImageManipulator"
        def description(self) -> str: return "Resizes and watermarks images."
        def is_available(self) -> bool: return False # Not available
        def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
            print(f"Dummy ImageManipulator running with: {input_data}")
            return {"status": "success", "message": "Dummy image manipulated."}

    registry = ToolRegistry()
    registry.register_tool(DummyEmailSender())
    registry.register_tool(DummyImageManipulator())

    print("--- Available Tool Descriptions ---")
    print(json.dumps(registry.get_available_tool_descriptions(), indent=2))

    email_tool = registry.get_tool_instance("EmailSender")
    if email_tool:
        print("\n--- Running EmailSender ---")
        email_tool.run({"to": "test@example.com", "subject": "Hello"})

    image_tool = registry.get_tool_instance("ImageManipulator")
    if image_tool:
        print("\n--- Running ImageManipulator ---")
        image_tool.run({"action": "resize"})
    else:
        print("\n--- ImageManipulator not available or found. ---")
