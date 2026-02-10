import os
from typing import Optional
from dotenv import load_dotenv
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import PydanticOutputParser
from langchain_google_genai import ChatGoogleGenerativeAI
from src.config import COMPANY_NAME, FLAGSHIP_PRODUCT
from src.data_models.design_spec import DesignSpecification

load_dotenv()

if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-pro-latest", temperature=0.7)

class CreativeDirectorAgent:
    def __init__(self):
        self.parser = PydanticOutputParser(pydantic_object=DesignSpecification)
        format_instructions = self.parser.get_format_instructions() # Define it here
        system_template = """You are an expert Creative Director for {company_name}. Your task is to provide detailed 'Design Specs' based on the provided campaign goal and research context. These specs must guide a human designer or an AI image generator, focusing on visual elements, tone, and overall composition.

            **CRITICAL INSTRUCTIONS:**
            1.  **Single Source of Truth:** You MUST treat the 'Research Context' and any 'Design System Specification' within it as the absolute and only source of truth. Do not invent, assume, or use any information not present in the context provided.
            2.  **Strict Adherence to Design System:** All generated design specifications MUST strictly adhere to established design standards (colors, typography, etc.) from the 'Research Context'. If no design system is provided, create one that is appropriate for a premium pet food brand (e.g., natural colors, clear typography).
            3.  **Output Format:** Your output MUST be a JSON string that conforms to the following Pydantic schema:

            ```json
            {{
                "overall_visual_concept": "A summary of the overall visual theme and mood.",
                "media_type": "The type of media to generate. Choose 'image' or 'video' based on the campaign goal. Prioritize video for dynamic storytelling or complex demonstrations. Prioritize image for static, high-impact visuals.",
                "image_prompt": "If media_type is 'image', provide the detailed prompt for image generation. This will be used directly by the image generator tool. Make it as descriptive as possible, including style, lighting, subjects, and composition. If media_type is 'video', this field should be null.",
                "video_script": "If media_type is 'video', provide a script or key messages for the video content. This should outline the narrative or key points to convey. If media_type is 'image', this field should be null.",
                "video_style": "If media_type is 'video', specify the visual style for the video (e.g., 'animated infographic', 'product showcase', 'slideshow with text overlays', 'testimonial style'). If media_type is 'image', this field should be null.",
                "video_duration_seconds": "If media_type is 'video', specify the desired duration of the video in seconds (e.g., 15, 30, 60). If media_type is 'image', this field should be null.",
                "video_assets_description": "If media_type is 'video', describe the assets needed for the video (e.g., 'stock footage of happy dogs', 'product shots', 'animated graphics'). If media_type is 'image', this field should be null.",
                "color_palette": ["List of HEX or named colors to be used in the design (e.g., ['#FFFFFF', '#0056b3']).",],
                "typography": "Guidance on typography (e.g., 'Headline: Montserrat Bold, Body: Lato Regular').",
                "layout_guidance": "Instructions on layout and composition (e.g., 'Single column, mobile-first, hero image top').",
                "ml_reasoning": "ML-enhanced reasoning justifying the design choices.",
                "proposed_design_change": "A proposal for improving the design system, if applicable."
            }}
            ```
            {format_instructions}
            """
        self.prompt = ChatPromptTemplate.from_messages([
            ("system", system_template),
            ("user", "Campaign Goal: {campaign_goal}\nResearch Context: {research_context}\n\nProvide Design Specifications:")
        ])
        self.chain = self.prompt | llm | self.parser

    def generate_design_specs(self, campaign_goal: str, research_context: str, reference_image_path: Optional[str] = None) -> DesignSpecification:
        """
        Generates design specifications based on the campaign goal and research context,
        optionally considering a reference image, returning a DesignSpecification object.
        """
        return self.chain.invoke({
            "campaign_goal": campaign_goal,
            "research_context": research_context,
            "format_instructions": self.parser.get_format_instructions(),
            "company_name": COMPANY_NAME,
            "flagship_product": FLAGSHIP_PRODUCT
        })

if __name__ == "__main__":
    # Example usage (for testing)
    # Ensure GOOGLE_API_KEY is set in your environment.
    creative_director = CreativeDirectorAgent()
    
    sample_campaign_goal = "Promote the new organic cat food 'Whiskers & Wellness Organic Salmon Feast' focusing on its health benefits in an email campaign."
    sample_research_context = """
    Product Name: Whiskers & Wellness Organic Salmon Feast
    Product Description: A premium organic cat food, specially formulated for adult cats. Made with wild-caught salmon, organic vegetables, and essential vitamins and minerals. Grain-free and hypoallergenic.
    Key Benefits: High in Omega-3 fatty acids for a healthy coat and skin, supports digestive health with prebiotics, made with 100% organic, human-grade ingredients, no artificial preservatives, colors, or flavors.
    Target Audience: Cat owners who prioritize organic, natural, and high-quality ingredients for their pets.
    Overall Tone: Nurturing, informative, trustworthy, and slightly playful.
    Key Message: We believe in providing pets with the best possible nutrition, mirroring the care and attention their human companions provide.
    Visuals: Images of happy, healthy cats, natural ingredients, clean and modern aesthetic.
    Design System Specification:
        Colors: Primary: #34495E (Dark Blue), Secondary: #E67E22 (Orange), Accent: #2ECC71 (Green)
        Typography: Headlines: 'Montserrat Bold', Body: 'Open Sans Regular'
    """
    
    print("\n--- Generating Design Specs for Email Campaign ---")
    try:
        design_specs_obj = creative_director.generate_design_specs(sample_campaign_goal, sample_research_context)
        print("Generated DesignSpecification:")
        print(design_specs_obj.json(indent=2))
    except Exception as e:
        print(f"Error generating design specs: {e}")
