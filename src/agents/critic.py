import os
from dotenv import load_dotenv
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import StrOutputParser
from langchain_google_genai import ChatGoogleGenerativeAI
from src.config import COMPANY_NAME, FLAGSHIP_PRODUCT

load_dotenv()

# Ensure GOOGLE_API_KEY is set as an environment variable
if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-2.5-pro", temperature=0.5)

class CriticAgent:
    def __init__(self):
        self.prompt = ChatPromptTemplate.from_messages([
            ("system", f"""You are an expert Marketing Critic for {COMPANY_NAME}. Your role is to provide constructive feedback on generated marketing content and design specifications. Your goal is to ensure the output is:
            1.  **Aligned with Campaign Goal:** Does it effectively meet the campaign's objective, including specific constraints (e.g., word count)?
            2.  **Grounded in Research Context:** Does it accurately use information from the provided research and adhere to brand/design standards?
            3.  **Compelling and Clear:** Is the language engaging, persuasive, and easy for the target audience to understand?
            4.  **Specific and Actionable:** Provide concrete suggestions for improvement. Always suggest revisions to *both* marketing content and design specs if necessary, even if one seems fine.
            5.  **Concise Output:** Your output MUST contain ONLY the constructive feedback. Do NOT include any conversational filler, introductory/concluding remarks, or explanations of your role. Just the feedback.

            **IMPORTANT:** Your feedback should be actionable and focused on iterative improvement. If the content is perfect and all constraints are met, state "No further improvements needed." Otherwise, always suggest revisions.
            """),
            ("user", "Campaign Goal: {campaign_goal}\nResearch Context: {research_context}\nMarketing Content: {marketing_content}\nDesign Specs: {design_specs}\n\nProvide constructive feedback for improvement, specifically noting if constraints from the Campaign Goal (e.g., word count) have been met:")
        ])
        self.chain = self.prompt | llm | StrOutputParser()

    def critique(self, campaign_goal: str, research_context: str, marketing_content: str, design_specs: str) -> str:
        """
        Provides critique on the generated marketing content and design specifications.
        """
        return self.chain.invoke({
            "campaign_goal": campaign_goal,
            "research_context": research_context,
            "marketing_content": marketing_content,
            "design_specs": design_specs
        })

if __name__ == "__main__":
    # Example usage
    critic = CriticAgent()
    sample_campaign_goal = "Promote 'Simply Raw®' beef for dogs, emphasizing health benefits."
    sample_research_context = """
    Product Name: Simply Raw® Beef (For Dogs)
    Key Benefits: High protein, easy digestion, shiny coat, increased energy.
    Brand Tone: Nurturing, expert.
    """
    sample_marketing_content = "Buy our dog food. It's good for your dog."
    sample_design_specs = "Image of dog eating food. Use blue colors."
    
    feedback = critic.critique(
        sample_campaign_goal,
        sample_research_context,
        sample_marketing_content,
        sample_design_specs
    )
    print("\n--- Critic Feedback ---")
    print(feedback)
