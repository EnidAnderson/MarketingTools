import os
from dotenv import load_dotenv
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import PydanticOutputParser
from langchain_google_genai import ChatGoogleGenerativeAI
from src.config import COMPANY_NAME, FLAGSHIP_PRODUCT
from typing import Optional, List
from src.data_models.social_media import SocialMediaPost, InstagramPost, FacebookPost, SocialMediaPostDraft

load_dotenv()

if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-pro-latest", temperature=0.7)

class CopywriterAgent:
    def __init__(self):
        self.parser = PydanticOutputParser(pydantic_object=SocialMediaPostDraft)
        system_template = """You are an expert Marketing Copywriter for {company_name}. Your task is to draft compelling marketing content for a social media post based on the provided campaign goal and research context.

            **CRITICAL INSTRUCTIONS:**
            1.  **Single Source of Truth:** You MUST treat the 'Research Context' as the absolute and only source of truth. Do not invent, assume, or use any information not present in the context provided.
            2.  **Specificity is Key:** Your copy must be specific and directly reference the key benefits, ingredients, and other details found in the 'Research Context' to create helpful, non-generic content for customers.
            3.  **Adhere to Naming Conventions:** Always refer to the company as "{company_name}" and our flagship product as "{flagship_product}".
            4.  **Output Format:** Your output MUST be a JSON string that conforms to the following Pydantic schema:
            
            ```json
            {{
                "platform": "The social media platform (e.g., 'instagram', 'facebook'). Based on the campaign goal, choose the most appropriate platform. Prioritize 'instagram' if visual content is key.",
                "text_content": "The main text content/caption of the post. Adhere to character limits if known, but focus on compelling copy.",
                "hashtags": ["List of relevant hashtags."],
                "mentions": ["List of user mentions (e.g., '@naturesdiet')."],
                "call_to_action_text": "Optional call to action text (e.g., 'Shop Now', 'Learn More').",
                "call_to_action_url": "Optional URL for the call to action."
            }}
            ```
            **Note:** You are only responsible for `platform`, `text_content`, `hashtags`, `mentions`, `call_to_action_text`, and `call_to_action_url`. Do NOT include `image_paths` or `video_path` as these will be handled by the Creative Director.

            {format_instructions}
            """
        self.prompt = ChatPromptTemplate.from_messages([
            ("system", system_template),
            ("user", "Campaign Goal: {campaign_goal}\nResearch Context: {research_context}\n\nDraft marketing content for a social media post:")
        ])
        self.chain = self.prompt | llm | self.parser

    def draft_content(self, campaign_goal: str, research_context: str) -> SocialMediaPostDraft:
        """
        Drafts marketing content based on the campaign goal and research context, returning a SocialMediaPostDraft object.
        """
        return self.chain.invoke({
            "campaign_goal": campaign_goal,
            "research_context": research_context,
            "format_instructions": self.parser.get_format_instructions(),
            "company_name": COMPANY_NAME,
            "flagship_product": FLAGSHIP_PRODUCT
        })

if __name__ == "__main__":
    copywriter = CopywriterAgent()
    
    sample_campaign_goal = "Promote the new organic cat food 'Whiskers & Wellness Organic Salmon Feast' focusing on its health benefits for Instagram."
    sample_research_context = """
    Product Name: Whiskers & Wellness Organic Salmon Feast
    Product Description: A premium organic cat food, specially formulated for adult cats. Made with wild-caught salmon, organic vegetables, and essential vitamins and minerals. Grain-free and hypoallergenic.
    Key Benefits: High in Omega-3 fatty acids for a healthy coat and skin, supports digestive health with prebiotics, made with 100% organic, human-grade ingredients, no artificial preservatives, colors, or flavors.
    Target Audience: Cat owners who prioritize organic, natural, and high-quality ingredients for their pets.
    Overall Tone: Nurturing, informative, trustworthy, and slightly playful.
    Key Message: We believe in providing pets with the best possible nutrition, mirroring the care and attention their human companions provide.
    """
    
    print("\n--- Drafting Instagram Post Content ---")
    try:
        insta_post_draft = copywriter.draft_content(sample_campaign_goal, sample_research_context)
        print("Generated SocialMediaPostDraft:")
        print(insta_post_draft.json(indent=2))
    except Exception as e:
        print(f"Error drafting content: {e}")

    sample_campaign_goal_fb = "Announce a new blog post about 'The Benefits of Simply Raw for Sensitive Stomachs' for Facebook."
    sample_research_context_fb = """
    Blog Post Title: The Benefits of Simply Raw for Sensitive Stomachs
    Key Points: Simply Raw is easily digestible, contains limited ingredients, and is ideal for pets with food sensitivities. Features real meat and vegetables, no fillers.
    Call to action: Read the full blog post on our website.
    """
    print("\n--- Drafting Facebook Post Content ---")
    try:
        fb_post_draft = copywriter.draft_content(sample_campaign_goal_fb, sample_research_context_fb)
        print("Generated SocialMediaPostDraft:")
        print(fb_post_draft.json(indent=2))
    except Exception as e:
        print(f"Error drafting content: {e}")
