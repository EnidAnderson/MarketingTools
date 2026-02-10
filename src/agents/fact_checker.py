import os
from google import genai
from dotenv import load_dotenv
from typing import List, Dict, Any, Optional
import json

from src.config import TRUSTED_URLS, PROJECT_ROOT
from src.utils.product_catalog import ProductCatalog
# Assuming these tools exist and are accessible through the workflow
# (e.g., provided globally by the execution environment or framework)

load_dotenv()

class FactCheckerAgent:
    def __init__(self, model_name: str = "gemini-2.5-pro"):
        """
        Initializes the FactCheckerAgent.
        Args:
            model_name (str): The name of the generative model to use.
        """
        api_key = os.getenv("GEMINI_API_KEY") or os.getenv("GOOGLE_API_KEY")
        if not api_key:
            raise ValueError("GEMINI_API_KEY or GOOGLE_API_KEY environment variable not set.")
        
        self.client = genai.Client(api_key=api_key)
        self.model_name = model_name
        
        # Initialize ProductCatalog
        products_dir = os.path.join(PROJECT_ROOT, "src", "data", "products")
        self.product_catalog = ProductCatalog(products_dir)


    def _extract_claims(self, content: str) -> List[str]:
        """Uses LLM to extract factual claims from the given content."""
        prompt = f"""
        Extract all distinct factual claims from the following content. A factual claim is a statement that can be proven true or false.
        List each claim on a new line. Do NOT include any conversational filler or explanations.

        Content:
        {content}

        Factual Claims:
        """
        response = self.client.models.generate_content(model=self.model_name, contents=[prompt])
        claims_text = response.text
        return [claim.strip() for claim in claims_text.split('\n') if claim.strip()]

    def _extract_and_verify_products(self, content: str) -> List[Dict[str, Any]]:
        """
        Uses LLM to extract potential product names from content and verifies their existence.
        """
        prompt = f"""
        From the following content, identify any potential product names (e.g., brand names, specific product lines, or product names with trademarks like ®).
        List each identified product name on a new line. Do NOT include any conversational filler or explanations.

        Content:
        {content}

        Identified Product Names:
        """
        response = self.client.models.generate_content(model=self.model_name, contents=[prompt])
        identified_products_text = response.text
        potential_product_names = [name.strip() for name in identified_products_text.split('\n') if name.strip()]

        verified_products = []
        for product_name in potential_product_names:
            exists = self.product_catalog.product_exists(product_name)
            if not exists:
                similar_product = self.product_catalog.find_similar_product(product_name)
                verified_products.append({
                    "name": product_name,
                    "exists": False,
                    "feedback": f"Product '{product_name}' does not exist in the official product catalog.",
                    "suggestion": f"Consider using a real product like '{similar_product}'" if similar_product else "No similar product found."
                })
            else:
                verified_products.append({
                    "name": product_name,
                    "exists": True,
                    "feedback": f"Product '{product_name}' verified in the official product catalog."
                })
        return verified_products


    def check_facts(self, campaign_goal: str, marketing_content: str, design_specs: str, research_context: str) -> str:
        """
        Fact-checks marketing content and design specs, providing a report with verified claims.
        Focuses on product existence verification.
        """
        print("--- FactCheckerAgent checking facts ---")
        full_report_md = "# Fact-Checking Report\n\n"

        # Product Existence Verification
        all_content_for_products = f"Campaign Goal: {campaign_goal}\nMarketing Content: {marketing_content}\nDesign Specs: {design_specs}\nResearch Context: {research_context}"
        product_checks = self._extract_and_verify_products(all_content_for_products)
        
        full_report_md += "## Product Existence Verification\n\n"
        if not product_checks:
            full_report_md += "No explicit product mentions found to verify.\n"
        else:
            for product_check in product_checks:
                status = "✅ Exists" if product_check["exists"] else "❌ Does NOT exist"
                full_report_md += f"- **Product Name:** {product_check['name']} - **Status:** {status}\n"
                if not product_check["exists"]:
                    full_report_md += f"  **Feedback:** {product_check['feedback']}\n"
                    if product_check["suggestion"]:
                        full_report_md += f"  **Suggestion:** {product_check['suggestion']}\n"
                full_report_md += "\n"
        
        print("Fact-checking complete.")
        return full_report_md
