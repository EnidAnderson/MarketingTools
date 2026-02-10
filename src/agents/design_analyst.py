import os
from dotenv import load_dotenv
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import StrOutputParser
from langchain_google_genai import ChatGoogleGenerativeAI

from src.tools.screenshot_tool import take_screenshot
from src.tools.css_analyzer import get_all_css
from src.config import PROJECT_ROOT, SCREENSHOTS_DIR, DATA_PATH

load_dotenv()

# Ensure GOOGLE_API_KEY is set as an environment variable
if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-2.5-pro", temperature=0.2)

class DesignAnalystAgent:
    def __init__(self):
        self.prompt = ChatPromptTemplate.from_messages([
            ("system", """You are an expert Design Analyst AI for Nature's Diet®. Your task is to analyze a website's design based on its rendered screenshot and extracted CSS rules. Your goal is to fill out the provided design system template with precise and semantic information.

            Analyze the visual elements from the screenshot and cross-reference them with the CSS to identify exact values for colors, fonts, spacing, and component styles.

            **Instructions:**
            1.  Extract details for each section (Brand Identity, Color Palette, Typography, Layout & Spacing, UI Components, Imagery & Iconography).
            2.  For colors, provide Hex and RGB values.
            3.  For typography, identify font families, sizes (e.g., in px or rem), and weights for different elements (H1, H2, body text, CTA).
            4.  For layout and spacing, identify page width, grid system if apparent, and consistent spacing units.
            5.  For UI components, describe the styling of buttons, input fields, and navigation links, including hover states where possible.
            6.  For imagery and iconography, describe the general style, color tone and type.
            7.  If information is not clearly identifiable, use "N/A" for that specific sub-field.
            8.  Ensure the output adheres strictly to the Markdown template provided.
            """),
            ("user", "Design System Template:\n{template}\n\nWebsite URL: {url}\nScreenshot Description: {screenshot_description}\nExtracted CSS:\n{css_content}\n\nFill out the Design System Template based on the above information:")
        ])
        self.chain = self.prompt | llm | StrOutputParser()

    def analyze_design(self, url: str, template_path: str) -> str:
        """
        Analyzes the design of a website and fills out a design system template.
        """
        # Ensure the screenshots directory exists
        os.makedirs(SCREENSHOTS_DIR, exist_ok=True)
        screenshot_filename = os.path.join(SCREENSHOTS_DIR, f"{url.replace('https://', '').replace('/', '_')}.png")
        take_screenshot(url, screenshot_filename)
        # Note: We are not passing the actual image to the LLM directly, but will describe it.
        # This is a simplification for now. Advanced multi-modal would involve sending the image.
        screenshot_description = f"Screenshot of {url} saved to {screenshot_filename}"

        # Get all CSS
        css_content = get_all_css(url)

        # Read the design system template
        with open(template_path, 'r') as f:
            design_template = f.read()

        # Call LLM to fill the template
        filled_design_system = self.chain.invoke({
            "template": design_template,
            "url": url,
            "screenshot_description": screenshot_description, # Providing description instead of image
            "css_content": css_content
        })
        
        return filled_design_system

if __name__ == "__main__":
    # Example usage
    design_analyst = DesignAnalystAgent()
    template_file = os.path.join(DATA_PATH, "design_system", "design_system_template.md")
    
    # Ensure the design_system directory exists
    output_design_system_dir = os.path.join(DATA_PATH, "design_system")
    os.makedirs(output_design_system_dir, exist_ok=True)

    filled_design_system_doc = design_analyst.analyze_design(
        url="https://www.naturesdietpet.com",
        template_path=template_file
    )
    
    output_file = os.path.join(output_design_system_dir, "current_design_system.md")
    with open(output_file, "w") as f:
        f.write(filled_design_system_doc)
    print(f"Filled Design System saved to: {output_file}")
