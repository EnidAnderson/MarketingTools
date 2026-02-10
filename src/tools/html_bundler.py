import os
import markdown
import json
import re
from playwright.sync_api import sync_playwright
from typing import Optional, List # Added for Pydantic type hints

from src.data_models.social_media import SocialMediaPost
from src.data_models.design_spec import DesignSpecification

def generate_html_report(
    campaign_dir: str,
    marketing_content: SocialMediaPost,
    design_specs: DesignSpecification,
    generated_image_path: Optional[str]
) -> str:
    """
    Generates a single HTML report from the campaign output Pydantic objects and image path.
    """
    report_html_path = os.path.join(campaign_dir, "report.html")
    
    # Read campaign goal for title (still from MD for simplicity here)
    campaign_goal_md = ""
    if os.path.exists(os.path.join(campaign_dir, "campaign_goal.md")):
        with open(os.path.join(campaign_dir, "campaign_goal.md"), "r") as f:
            campaign_goal_md = f.read()

    html_content = f"""
<!DOCTYPE html>
<html>
<head>
    <title>Campaign Output: {campaign_goal_md.splitlines()[0].replace('# Campaign Goal', '').strip() if campaign_goal_md.strip() else 'Untitled Campaign'}</title>
    <style>
        body {{ font-family: sans-serif; line-height: 1.6; color: #333; max-width: 960px; margin: 0 auto; padding: 20px; }}
        h1, h2, h3 {{ color: #0056b3; }}
        img {{ max-width: 100%; height: auto; display: block; margin: 20px 0; border: 1px solid #ddd; padding: 5px; background: #f8f8f8; }}
        .section {{ background-color: #f9f9f9; border-left: 5px solid #0056b3; margin-bottom: 20px; padding: 15px; border-radius: 5px; }}
        pre {{ background-color: #eef; padding: 10px; border-radius: 5px; overflow-x: auto; }}
    </style>
</head>
<body>
    <div class="section">
        <h1>Campaign Output</h1>
        <h2>Marketing Content ({marketing_content.platform.capitalize()})</h2>
        {markdown.markdown(marketing_content.text_content)}
        <h3>Hashtags:</h3>
        <p>{', '.join(marketing_content.hashtags)}</p>
        <h3>Mentions:</h3>
        <p>{', '.join(marketing_content.mentions)}</p>
        {marketing_content.call_to_action_text and f'<h3>Call to Action:</h3><p>{marketing_content.call_to_action_text} - <a href="{marketing_content.call_to_action_url}">{marketing_content.call_to_action_url}</a></p>' or ''}
    </div>

    <div class="section">
        <h2>Image Generation Prompt</h2>
        <pre>{design_specs.image_prompt}</pre>
    </div>

    <div class="section">
        <h2>Generated Image</h2>
        {generated_image_path and f'<img src="{os.path.basename(generated_image_path)}" alt="Generated Campaign Image">' or '<p>No image generated or found.</p>'}
    </div>

    <div class="section">
        <h2>Full Design Specifications (JSON)</h2>
        <pre>{design_specs.model_dump_json(indent=2)}</pre>
    </div>

</body>
</html>
    """.strip()

    with open(report_html_path, "w") as f:
        f.write(html_content)
        
    print(f"HTML report generated at: {report_html_path}")
    return report_html_path

def generate_pdf_report(html_path: str, pdf_path: str):
    """
    Converts an HTML file to a PDF file using Playwright.
    """
    print(f"Generating PDF report from {html_path} to {pdf_path}")
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch()
            page = browser.new_page()
            page.goto(f"file://{os.path.abspath(html_path)}")
            page.pdf(path=pdf_path)
            browser.close()
        print("PDF report generated successfully.")
        return pdf_path
    except Exception as e:
        print(f"Error generating PDF report: {e}")
        return None

if __name__ == '__main__':
    # Example usage for testing purposes
    # Create a dummy campaign directory and files
    dummy_campaign_dir = "CAMPAIGNS/dummy_test_campaign_report"
    os.makedirs(dummy_campaign_dir, exist_ok=True)

    with open(os.path.join(dummy_campaign_dir, "campaign_goal.md"), "w") as f:
        f.write("# Campaign Goal\n\nPromote new product X.")
    with open(os.path.join(dummy_campaign_dir, "research_context.md"), "w") as f:
        f.write("# Research Context\n\nMarket research data for product X.")
    with open(os.path.join(dummy_campaign_dir, "marketing_content_v1.md"), "w") as f:
        f.write("# Marketing Content Version 1\n\nDraft 1 content.")
    with open(os.path.join(dummy_campaign_dir, "marketing_content_FINAL.md"), "w") as f:
        f.write("# Marketing Content (Final)\n\nFinal marketing copy for product X.")
    with open(os.path.join(dummy_campaign_dir, "design_specs_FINAL.md"), "w") as f:
        f.write("# Design Specifications (Final)\n\nVisuals: Product X on white background.")
    
    # Create a dummy image
    from PIL import Image
    dummy_image_path = os.path.join(dummy_campaign_dir, "dummy_image_humanized.png")
    Image.new('RGB', (100, 100), color = 'red').save(dummy_image_path)
    # with open(os.path.join(dummy_campaign_dir, "generated_image.md"), "w") as f:
    #     f.write(f"# Generated Image\n\nImage saved to: {dummy_image_path}") # This is no longer read

    # Create dummy Pydantic objects for the example
    dummy_marketing_content = SocialMediaPost(
        platform="instagram",
        text_content="This is a dummy marketing content. #dummy #example",
        image_paths=[],
        hashtags=["dummy", "example"],
        mentions=[],
    )
    dummy_design_specs = DesignSpecification(
        overall_visual_concept="Dummy visual concept.",
        image_prompt="Dummy image prompt for a test image.",
        color_palette=["#FF0000", "#00FF00"],
        typography="Dummy font.",
        layout_guidance="Dummy layout.",
        ml_reasoning="Dummy reasoning."
    )

    report_path = generate_html_report(
        campaign_dir=dummy_campaign_dir,
        marketing_content=dummy_marketing_content,
        design_specs=dummy_design_specs,
        generated_image_path=dummy_image_path
    )
    print(f"Dummy report generated: {report_path}")
