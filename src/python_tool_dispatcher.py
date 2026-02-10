import sys
import importlib
import json
import os

# Add the parent directory of src/tools to the Python path
# This allows importing tools like `from tools import email_sender_tool`
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

def dispatch_tool_call(tool_name: str, params_json: str):
    """
    Dispatches a call to a specified Python tool with given parameters.
    """
    try:
        # Construct the module path for the tool
        # Assuming tools are in src/tools/ and are named like tool_name.py
        module_path = f"src.tools.{tool_name}"
        tool_module = importlib.import_module(module_path)

        # Assuming the tool class or function has the same name as the tool_name (e.g., EmailSenderTool in email_sender_tool.py)
        # Or a common 'run' function
        tool_class_name = ''.join(word.capitalize() for word in tool_name.split('_'))
        tool_instance = getattr(tool_module, tool_class_name)() # Assuming it's a class and needs instantiation

        params = json.loads(params_json)

        # Call the tool's run method
        result = tool_instance.run(**params)
        print(json.dumps({"status": "success", "result": result}))

    except ModuleNotFoundError:
        print(json.dumps({"status": "error", "message": f"Tool '{tool_name}' not found."}))
    except AttributeError:
        print(json.dumps({"status": "error", "message": f"Tool '{tool_name}' does not have a callable 'run' method or the class name is incorrect."}))
    except json.JSONDecodeError:
        print(json.dumps({"status": "error", "message": "Invalid JSON parameters provided."}))
    except Exception as e:
        print(json.dumps({"status": "error", "message": f"An unexpected error occurred: {str(e)}"}))

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(json.dumps({"status": "error", "message": "Usage: python python_tool_dispatcher.py <tool_name> <json_parameters>"}))
        sys.exit(1)

    tool_name = sys.argv[1]
    params_json = sys.argv[2]

    dispatch_tool_call(tool_name, params_json)
