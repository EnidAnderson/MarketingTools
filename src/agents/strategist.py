import os
from dotenv import load_dotenv # Added import
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import StrOutputParser
from langchain_google_genai import ChatGoogleGenerativeAI

load_dotenv() # Load environment variables from .env

# Ensure GOOGLE_API_KEY is set as an environment variable
# You can get one from https://ai.google.dev/
if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-pro-latest", temperature=0.7)

class StrategistAgent:
    def __init__(self):
        self.prompt = ChatPromptTemplate.from_messages([
            ("system", """You are an expert Marketing Strategist. Your goal is to analyze a high-level marketing objective and refine it into a clear, specific, and actionable campaign goal. 
            The goal you define will be used as a query for a vector store to retrieve specific, factual information about our products and brand. 
            Therefore, ensure your refined goal is concise, rich in keywords, and directly relevant to the user's objective, enabling effective downstream information retrieval.
            """),
            ("user", "Refine the following marketing objective into a concise campaign goal: {objective}")
        ])
        self.chain = self.prompt | llm | StrOutputParser()

    def refine_goal(self, objective: str) -> str:
        """
        Refines a high-level marketing objective into a specific campaign goal.
        """
        return self.chain.invoke({"objective": objective})

if __name__ == "__main__":
    # This is an example of how to use the StrategistAgent.
    # For actual execution, ensure GOOGLE_API_KEY is set in your environment.
    strategist = StrategistAgent()
    marketing_objective = "I want to promote our new line of organic cat food to new customers."
    refined_goal = strategist.refine_goal(marketing_objective)
    print(f"Original Objective: {marketing_objective}")
    print(f"Refined Campaign Goal: {refined_goal}")
