import argparse
import os
from datetime import datetime

from src.graph import create_multi_post_workflow, AgentState
from src.tools.generation_budget_manager import get_budget_status

def main():
    parser = argparse.ArgumentParser(description="Run a multi-post social media campaign.")
    parser.add_argument(
        "--overall_campaign_goal",
        type=str,
        required=True,
        help="The overarching goal for the entire multi-post campaign.",
    )
    parser.add_argument(
        "--target_audience",
        type=str,
        required=True,
        help="Description of the target audience for the campaign.",
    )
    parser.add_argument(
        "--num_posts",
        type=int,
        default=20,
        help="The desired number of social media posts to generate (default: 20).",
    )
    parser.add_argument(
        "--output_dir",
        type=str,
        default=None,
        help="Optional: The base directory to save all campaign outputs. Defaults to CAMPAIGNS/multi_post_<timestamp>.",
    )
    parser.add_argument(
        "--word_length",
        type=int,
        default=None,
        help="Optional: Desired word length for marketing content.",
    )
    parser.add_argument(
        "--reference_image_path",
        type=str,
        default=None,
        help="Optional: Path to a reference image to be used by image generation tools.",
    )
    parser.add_argument(
        "--no_copywriting",
        action="store_true",
        help="Optional: Flag to skip copywriting agents and focus on image generation/design specs.",
    )

    args = parser.parse_args()

    # Create output directory
    if args.output_dir:
        multi_post_campaign_dir = args.output_dir
    else:
        timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        multi_post_campaign_dir = os.path.join("CAMPAIGNS", f"multi_post_{timestamp}")
    
    os.makedirs(multi_post_campaign_dir, exist_ok=True)
    print(f"Campaign outputs will be saved to: {multi_post_campaign_dir}")

    multi_post_app = create_multi_post_workflow()

    initial_multi_post_state: AgentState = {
        "overall_campaign_goal": args.overall_campaign_goal,
        "target_audience": args.target_audience,
        "num_posts": args.num_posts,
        "campaign_dir": multi_post_campaign_dir, # This is the overall campaign directory
        "overall_campaign_dir": multi_post_campaign_dir, # Also pass this for final report node
        "post_objectives": [], # Will be populated by plan_posts_node
        "current_post_index": 0, # Initialized to 0
        "all_post_results": [], # Initialized as empty list
        "cumulative_daily_spend": get_budget_status(), # Initial budget status
        "last_action_cost": 0,
        
        # Default values for single_post_workflow, will be overwritten by prepare_single_post_state_node
        "campaign_goal": "", 
        "research_context": "",
        "marketing_content": "",
        "design_specs": "",
        "generated_image_path": "",
        "proposed_design_change": "",
        "html_report_path": None,
        "pdf_report_path": None,
        "word_length": args.word_length,
        "reference_image_path": args.reference_image_path,
        "no_copywriting": args.no_copywriting,
        "marketing_content_versions": [],
        "design_specs_versions": [],
        "critique_feedback": "",
        "iteration_count": 0,
    }

    print("\n--- Starting Multi-Post Campaign Generation ---")
    final_multi_post_state = multi_post_app.invoke(initial_multi_post_state)

    print("\n--- Final Multi-Post Campaign Summary ---")
    print(f"Overall Campaign Goal: {final_multi_post_state.get('overall_campaign_goal')}")
    print(f"Target Audience: {final_multi_post_state.get('target_audience')}")
    print(f"Total Posts Generated: {len(final_multi_post_state.get('all_post_results', []))}")
    
    for i, post_result in enumerate(final_multi_post_state.get('all_post_results', [])):
        print(f"\n--- Post {i+1} Details ---")
        print(f"  Objective: {post_result.get('post_objective')}")
        print(f"  Saved to: {post_result.get('campaign_dir')}")
        if post_result.get('generated_image_path'):
            print(f"  Image Path: {post_result.get('generated_image_path')}")
        if post_result.get('html_report_path'):
            print(f"  HTML Report: {post_result.get('html_report_path')}")
        if post_result.get('pdf_report_path'):
            print(f"  PDF Report: {post_result.get('pdf_report_path')}")
        
    print(f"\nFinal Summary Report: {final_multi_post_state.get('final_multi_post_report_path')}")
    print(f"Total Cumulative Spend for Campaign: {final_multi_post_state.get('cumulative_daily_spend')}")

if __name__ == "__main__":
    main()
