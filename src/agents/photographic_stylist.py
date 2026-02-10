import os
from google import genai
from dotenv import load_dotenv
from typing import Optional

# Load environment variables
load_dotenv()

class PhotographicStylist:
    def __init__(self, model_name: str = "gemini-2.5-flash"):
        """
        Initializes the PhotographicStylist agent.
        Args:
            model_name (str): The name of the generative model to use for prompt enrichment.
        """
        api_key = os.getenv("GEMINI_API_KEY") or os.getenv("GOOGLE_API_KEY")
        if not api_key:
            raise ValueError("GEMINI_API_KEY or GOOGLE_API_KEY environment variable not set.")
        
        self.client = genai.Client(api_key=api_key)
        self.model_name = model_name
        self.guidance_content = self._load_guidance()
        
    def _load_guidance(self) -> str:
        """Loads the content of the IMAGE_PROMPTING_GUIDE.md."""
        # Assuming the guide is in src/data relative to the project root
        # and this agent is in src/agents
        guide_path = os.path.join(
            os.path.dirname(os.path.dirname(__file__)), "data", "IMAGE_PROMPTING_GUIDE.md"
        )
        if not os.path.exists(guide_path):
            raise FileNotFoundError(f"IMAGE_PROMPTING_GUIDE.md not found at {guide_path}")
        with open(guide_path, "r") as f:
            return f.read()

    def enrich_prompt(self, base_prompt: str, artistic_direction: Optional[str] = None) -> str:
        """
        Enriches a base image prompt with detailed photographic terminology
        based on the loaded guidance.
        
        Args:
            base_prompt (str): The initial, concise image prompt.
            artistic_direction (Optional[str]): Optional high-level artistic direction
                                                (e.g., "cinematic", "natural lighting").
        Returns:
            str: The enriched prompt string.
        """
        print("Photographic Stylist enriching prompt...")
        
        # Construct the system instruction/context for the LLM
        system_instruction = f"""
        You are an expert Photographic Stylist AI. Your task is to take a concise image generation request
        and expand it into a highly detailed, photography-specific prompt suitable for a Diffusion model
        like Stable Diffusion.
        
        Use the following guidance to inform your prompt enrichment. Focus on adding details related to:
        - Camera & Lens Specificity
        - Lighting & Shadow Control
        - Texture & Surface Qualities
        - Composition & Framing
        - Overall Aesthetic & Style References
        - Appropriate Negative Prompts

        --- Guidance Document ---
        {self.guidance_content}
        --- End Guidance Document ---

        Ensure the output is a single, continuous prompt string.
        Include a "--neg" section for negative prompts at the end of the enriched prompt.
        Prioritize realism and artistic quality based on photographic best practices.
        """
        
        user_message = f"Base Prompt: {base_prompt}\n"
        if artistic_direction:
            user_message += f"Artistic Direction: {artistic_direction}\n"
        user_message += "Please provide the enriched image prompt:"
        
        full_prompt = f"{system_instruction}\n\n{user_message}"
        
        response = self.client.models.generate_content(
            model=self.model_name,
            contents=[full_prompt]
        )
        
        enriched_prompt = response.text
        print("Prompt enriched successfully.")
        return enriched_prompt

if __name__ == '__main__':
    # Example usage
    stylist = PhotographicStylist()
    
    base_prompt_example = "A happy golden retriever playing with a toy"
    artistic_direction_example = "cinematic, warm tones"
    
    enriched = stylist.enrich_prompt(base_prompt_example, artistic_direction_example)
    print(f"\n--- Original Prompt ---\n{base_prompt_example}")
    print(f"\n--- Enriched Prompt ---\n{enriched}")

    base_prompt_example_2 = "A futuristic city at night"
    enriched_2 = stylist.enrich_prompt(base_prompt_example_2)
    print(f"\n--- Original Prompt ---\n{base_prompt_example_2}")
    print(f"\n--- Enriched Prompt ---\n{enriched_2}")
