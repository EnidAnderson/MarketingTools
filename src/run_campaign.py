import argparse
import os
from datetime import datetime
import re
from typing import Optional # Added import
from dotenv import load_dotenv

from src.graph import create_workflow

load_dotenv()

def sanitize_filename(text):
    """Sanitizes text to be used as a filename."""
    text = re.sub(r'[^\w\s-]', '', text).strip()
    text = re.sub(r'[-\s]+', '-', text)
    return text.lower()

from src.tools.generation_budget_manager import get_budget_status

def run_campaign(objective: str, word_length: Optional[int] = None, image_path: Optional[str] = None, no_copywriting: bool = False):
    """
    Runs a marketing campaign using the LangGraph workflow and saves the output.
    """
    app = create_workflow()
    
    # Create a unique directory for the campaign output upfront
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    # Sanitize the objective for the directory name
    sanitized_objective = sanitize_filename(objective)
    campaign_name = f"{timestamp}-{sanitized_objective[:50]}"
    output_dir = os.path.join("CAMPAIGNS", campaign_name)
    os.makedirs(output_dir, exist_ok=True)
    
    print(f"Starting campaign for objective: '{objective}'")
    if image_path:
        print(f"Using reference image: {image_path}")
    if no_copywriting:
        print("Skipping copywriting and content generation steps.")
    print(f"Campaign directory: {output_dir}")
    
    initial_state = {
        "campaign_goal": objective,
        "campaign_dir": output_dir,
        "last_action_cost": 0.0,
        "cumulative_daily_spend": get_budget_status(),
        "word_length": word_length, # Pass word_length to initial state
        "reference_image_path": image_path, # Pass image_path to initial state
        "no_copywriting": no_copywriting, # Pass no_copywriting flag
    }
    
    final_state = app.invoke(initial_state, config={"recursion_limit": 100})
    return final_state

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run a Nature's Diet Pet marketing campaign.")
    parser.add_argument("objective", type=str, help="The high-level marketing objective for the campaign.")
    parser.add_argument("--word_length", type=int, help="Optional: Desired word length for the marketing copy.")
    parser.add_argument("--image-path", type=str, help="Optional: Path to a reference image for image generation.")
    parser.add_argument("--no-copywriting", action="store_true", help="Optional: Skip copywriting and content generation steps, focusing only on image generation.")
    args = parser.parse_args()
    
    final_state_from_run = run_campaign(args.objective, args.word_length, args.image_path, args.no_copywriting) # Capture the final state

    # Now print a concise summary using the returned final_state
    print("\n--- Campaign Execution Complete ---")
    print(f"Campaign Objective: {args.objective}")
    print(f"Output Directory: {final_state_from_run['campaign_dir']}")

    if final_state_from_run.get('generated_image_path'):
        print(f"Generated Image: {final_state_from_run['generated_image_path']}")
    if final_state_from_run.get('html_report_path'):
        print(f"HTML Report: {final_state_from_run['html_report_path']}")
    if final_state_from_run.get('pdf_report_path'):
        print(f"PDF Report: {final_state_from_run['pdf_report_path']}")
    
    print(f"Final Cumulative Daily Spend: {final_state_from_run.get('cumulative_daily_spend')}")