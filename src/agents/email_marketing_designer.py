import os
import re
from google import genai
from dotenv import load_dotenv
from typing import Optional

load_dotenv()

class EmailMarketingDesignerAgent:
    def __init__(self, model_name: str = "gemini-pro"):
        """
        Initializes the EmailMarketingDesignerAgent.
        Args:
            model_name (str): The name of the generative model to use.
        """
        api_key = os.getenv("GEMINI_API_KEY") or os.getenv("GOOGLE_API_KEY")
        if not api_key:
            raise ValueError("GEMINI_API_KEY or GOOGLE_API_KEY environment variable not set.")
        
        self.client = genai.Client(api_key=api_key)
        self.model_name = model_name
        
    def _read_file_content(self, file_path: str, default_content: str = "") -> str:
        """Helper to read file content safely."""
        if os.path.exists(file_path):
            with open(file_path, "r") as f:
                return f.read()
        return default_content

    def generate_email_html(self, state: dict) -> str:
        """
        Generates email-friendly HTML based on campaign content, design specs, and image.
        """
        print("--- Email Marketing Designer generating email HTML ---")
        campaign_dir = state["campaign_dir"]
        
        # Read final content and specs
        campaign_goal_md = self._read_file_content(os.path.join(campaign_dir, "campaign_goal.md"))
        research_context_md = self._read_file_content(os.path.join(campaign_dir, "research_context.md"))
        marketing_content_final_md = self._read_file_content(os.path.join(campaign_dir, "marketing_content_FINAL.md"))
        design_specs_final_md = self._read_file_content(os.path.join(campaign_dir, "design_specs_FINAL.md"))
        
        # Extract generated image path from its markdown file
        generated_image_md_content = self._read_file_content(os.path.join(campaign_dir, "generated_image.md"))
        generated_image_path = None
        if "Image saved to:" in generated_image_md_content:
            # Need to convert to relative path for HTML embedding
            absolute_image_path = generated_image_md_content.split("Image saved to:")[1].strip()
            generated_image_path = os.path.basename(absolute_image_path)
        
        # --- Prompt for LLM to generate Email HTML ---
        system_instruction = f"""
        You are an expert Email Marketing Designer. Your task is to generate a single, complete,
        email-friendly HTML file based on the provided campaign content, design specifications,
        and image path.

        **CRITICAL INSTRUCTIONS for HTML Generation:**
        1.  **Email Compatibility:** All CSS MUST be inlined. Use a table-based layout or modern, email-compatible CSS. Ensure cross-client compatibility.
        2.  **Responsiveness:** Implement basic responsiveness (e.g., fluid images, max-width on containers).
        3.  **Content Integration:** Integrate the 'Final Marketing Content' as the main body. Extract key visual and style elements from 'Final Design Specifications' and apply them.
        4.  **Image Integration:** Reference the generated image using its filename. Assume the image is in the same directory as the HTML.
        5.  **Exclusion of Extraneous Content:** Your output MUST ONLY contain the HTML code. Do NOT include any conversational filler like "Here is the HTML:", "```html", or any explanations. Just the raw HTML.
        6.  **HTML Structure:** Include standard email boilerplate (`<!DOCTYPE html>`, `<html>`, `<head>`, `<body>`).
        7.  **Call to Action:** Implicitly create a call to action button if suggested in marketing content or based on common email practices.
        8.  **Clean Content:** The Marketing Content might contain conversational filler from previous LLM interactions (e.g., "Here is the content:"). Remove such filler before integrating.
        """
        
        user_message = f"""
        **Campaign Goal:**
        {campaign_goal_md}

        **Research Context:**
        {research_context_md}

        **Final Marketing Content:**
        {marketing_content_final_md}

        **Final Design Specifications:**
        {design_specs_final_md}

        **Generated Image Filename (assume same directory as HTML):**
        {generated_image_path if generated_image_path else 'No image generated.'}

        Please generate the complete, email-friendly HTML for this campaign.
        """
        
        full_prompt = f"{system_instruction}\n\n{user_message}"

        response = self.client.models.generate_content(
            model=self.model_name,
            contents=[full_prompt]
        )
        
        email_html_content = response.text
        
        # Post-process to remove potential ```html wraps if LLM adds them
        if email_html_content.strip().startswith("```html"):
            email_html_content = email_html_content.strip()[len("```html"):]
            if email_html_content.strip().endswith("```"):
                email_html_content = email_html_content.strip()[:-len("```")]

        email_html_path = os.path.join(campaign_dir, "email_campaign.html")
        with open(email_html_path, "w") as f:
            f.write(email_html_content.strip())
            
        print(f"Email HTML generated at: {email_html_path}")
        return email_html_path

if __name__ == '__main__':
    # Example usage (for testing)
    # This will require GOOGLE_API_KEY to be set
    
    # Create a dummy campaign directory and files
    dummy_campaign_dir = "CAMPAIGNS/dummy_email_campaign_report"
    os.makedirs(dummy_campaign_dir, exist_ok=True)

    # Create dummy markdown files
    with open(os.path.join(dummy_campaign_dir, "campaign_goal.md"), "w") as f:
        f.write("# Campaign Goal\n\nPromote new product X.")
    with open(os.path.join(dummy_campaign_dir, "research_context.md"), "w") as f:
        f.write("# Research Context\n\nMarket research data for product X.")
    with open(os.path.join(dummy_campaign_dir, "marketing_content_FINAL.md"), "w") as f:
        f.write("# Marketing Content (Final)\n\nIntroducing our amazing new Super Salmon cat food! Give your feline friend the best. Learn more!")
    with open(os.path.join(dummy_campaign_dir, "design_specs_FINAL.md"), "w") as f:
        f.write("# Design Specifications (Final)\n\nVisuals: image of happy cat. Colors: vibrant blue (#0000FF), accent yellow (#FFFF00). Font: Arial, sans-serif. Tone: Playful yet premium.")
    
    # Create a dummy image
    from PIL import Image
    dummy_image_path = os.path.join(dummy_campaign_dir, "dummy_cat_food.png")
    Image.new('RGB', (400, 300), color = 'blue').save(dummy_image_path)
    with open(os.path.join(dummy_campaign_dir, "generated_image.md"), "w") as f:
        f.write(f"# Generated Image\n\nImage saved to: {dummy_image_path}")

    # Run the agent
    designer = EmailMarketingDesignerAgent()
    email_path = designer.generate_email_html({"campaign_dir": dummy_campaign_dir})
    print(f"Generated email HTML: {email_path}")
