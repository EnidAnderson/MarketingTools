import json
import os
from datetime import datetime, timedelta, date
import src.config as config

# Global variable to store API_COSTS, loaded once
API_COSTS = {}
API_COSTS_FILE = os.path.join(
    os.path.dirname(os.path.dirname(__file__)), "data", "api_costs.json"
)

def _load_api_costs():
    """Loads API costs from a JSON file."""
    global API_COSTS
    if not os.path.exists(API_COSTS_FILE):
        print(f"Warning: API costs file not found at {API_COSTS_FILE}. Using empty costs.")
        API_COSTS = {}
    else:
        with open(API_COSTS_FILE, 'r') as f:
            API_COSTS = json.load(f)

# Load costs when the module is imported
_load_api_costs()

def get_budget_state():
    """Loads the budget state from the JSON file."""
    if not os.path.exists(config.BUDGET_FILE):
        return {
            "daily_spend": 0.0,
            "daily_resets_on": str(date.today() + timedelta(days=1)),
            "generations": []
        }
    with open(config.BUDGET_FILE, 'r') as f:
        return json.load(f)

def save_budget_state(state):
    """Saves the budget state to the JSON file."""
    with open(config.BUDGET_FILE, 'w') as f:
        json.dump(state, f, indent=4)

def get_next_month():
    """Returns the first day of the next month."""
    today = datetime.today()
    if today.month == 12:
        return today.replace(year=today.year + 1, month=1, day=1).isoformat()
    else:
        return today.replace(month=today.month + 1, day=1).isoformat()

def check_and_update_budget():
    """
    Checks if the daily budget needs to be reset.
    If the current date is past the reset date, it resets the daily spend.
    """
    state = get_budget_state()
    today_str = str(date.today())
    
    # Use ">=" to handle cases where the script hasn't run for multiple days
    if today_str >= state.get("daily_resets_on", today_str):
        print("Resetting daily budget.")
        state["daily_spend"] = 0.0
        state["daily_resets_on"] = str(date.today() + timedelta(days=1))
        # Also clear old generations to keep the file size manageable
        one_month_ago = datetime.now() - timedelta(days=30)
        state["generations"] = [
            g for g in state.get("generations", []) 
            if datetime.fromisoformat(g['timestamp']) > one_month_ago
        ]
        save_budget_state(state)
    return state

def estimate_llm_cost(model_name: str, input_text: str, output_text: str = "") -> float: # Added default for output_text
    """Estimates the cost of an LLM call."""
    if model_name not in API_COSTS:
        return 0.0
    
    # Simple token estimation: 1 token ~ 4 characters
    input_tokens = len(input_text) / 4
    output_tokens = len(output_text) / 4
    
    cost = (input_tokens * API_COSTS[model_name]["input"]) + \
           (output_tokens * API_COSTS[model_name]["output"])
           
    return cost

def estimate_embedding_cost(model_name: str, text: str) -> float:
    """Estimates the cost of an embedding call."""
    if model_name not in API_COSTS: # Now uses globally loaded API_COSTS
        return 0.0
    
    # Simple token estimation: 1 token ~ 4 characters
    tokens = len(text) / 4
    
    # Embedding models usually only have an input cost
    cost = tokens * API_COSTS[model_name]["input"] # Now uses globally loaded API_COSTS
           
    return cost

def can_generate(cost: float):
    """
    Checks if a generation is allowed based on the daily budget.
    """
    state = check_and_update_budget()
    
    if state["daily_spend"] + cost > config.BUDGET_CONFIG["daily_budget_usd"]:
        print(f"Daily budget exceeded. Current spend: ${state['daily_spend']:.2f}")
        return False
        
    return True

def record_generation(cost: float, tool_name: str):
    """Records a new generation and its cost."""
    state = check_and_update_budget()
    
    state["daily_spend"] += cost
    state["generations"].append({
        "timestamp": datetime.now().isoformat(),
        "tool": tool_name,
        "cost": cost
    })
    
    save_budget_state(state)
    print(f"Recorded generation from '{tool_name}' with cost ${cost:.4f}.")

def get_budget_status():
    """Returns a string with the current budget status."""
    state = check_and_update_budget()
    daily_spend = state.get("daily_spend", 0.0)
    
    return (
        f"Daily spend: ${daily_spend:.4f} / ${config.BUDGET_CONFIG['daily_budget_usd']:.2f}"
    )

if __name__ == "__main__":
    # Example Usage
    print("--- Budget Status ---")
    print(get_budget_status())
    
    print("\n--- Checking if generation is possible ---")
    # Example cost for a small LLM call
    example_llm_cost = estimate_llm_cost("gemini-2.5-pro", "Hello", "Hi there!")
    example_embedding_cost = estimate_embedding_cost("models/embedding-001", "This is some text to embed.")
    
    if can_generate(example_llm_cost):
        print(f"LLM Generation is possible (cost: ${example_llm_cost:.4f}).")
        record_generation(example_llm_cost, "test_llm_tool")
        print("\n--- New Budget Status ---")
        print(get_budget_status())
    else:
        print(f"LLM Generation is not possible (cost: ${example_llm_cost:.4f}).")

    if can_generate(example_embedding_cost):
        print(f"Embedding Generation is possible (cost: ${example_embedding_cost:.4f}).")
        record_generation(example_embedding_cost, "test_embedding_tool")
        print("\n--- New Budget Status ---")
        print(get_budget_status())
    else:
        print(f"Embedding Generation is not possible (cost: ${example_embedding_cost:.4f}).")
