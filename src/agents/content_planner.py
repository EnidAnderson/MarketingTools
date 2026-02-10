import os
from dotenv import load_dotenv
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import StrOutputParser
from langchain_google_genai import ChatGoogleGenerativeAI
from typing import List

load_dotenv()

if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-pro-latest", temperature=0.7)

class ContentPlannerAgent:
    def __init__(self):
        self.prompt = ChatPromptTemplate.from_messages([
            ("system", """You are an expert Social Media Content Planner. Your task is to generate a list of distinct, engaging social media post objectives/themes based on a given overall campaign goal and target audience.

            Each objective should be concise, compelling, and designed to resonate with the target audience.
            The output MUST be a numbered list of objectives, with each objective on a new line. Do NOT include any additional text or formatting.
            Example:
            1. Objective one
            2. Objective two
            3. Objective three
            """,
            ),
            ("user", """Overall Campaign Goal: {campaign_goal}
            Target Audience: {target_audience}
            Number of posts: {num_posts}

            Generate a numbered list of {num_posts} social media post objectives/themes:
            """,
            )
        ])
        self.chain = self.prompt | llm | StrOutputParser()

    def plan_posts(self, campaign_goal: str, target_audience: str, num_posts: int) -> List[str]:
        """
        Generates a list of social media post objectives/themes.
        """
        response = self.chain.invoke({
            "campaign_goal": campaign_goal,
            "target_audience": target_audience,
            "num_posts": num_posts
        })
        # Parse the numbered list into a Python list of strings
        objectives = [line.split('.', 1)[1].strip() for line in response.split('\n') if line.strip() and line.strip()[0].isdigit()]
        return objectives

if __name__ == "__main__":
    planner = ContentPlannerAgent()
    overall_goal = "Create a social media campaign emphasizing why Simply Raw is great for beginners."
    audience = "New pet owners or those new to raw food, looking for affordable and convenient alternatives to frozen/fresh raw food."
    num = 5
    post_objectives = planner.plan_posts(overall_goal, audience, num)
    print(f"Generated {len(post_objectives)} post objectives:")
    for i, obj in enumerate(post_objectives):
        print(f"{i+1}. {obj}")
