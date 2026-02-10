from pydantic import BaseModel, Field
from typing import List, Optional, Literal # Added Literal

class DesignSpecification(BaseModel):
    """
    Data structure for detailed design specifications for a social media post,
    including options for video generation.
    """
    overall_visual_concept: str = Field(..., description="A summary of the overall visual theme and mood.")
    
    media_type: Literal["image", "video"] = Field("image", description="The type of media to generate. Defaults to 'image'.")
    image_prompt: Optional[str] = Field(None, description="The detailed prompt for image generation. This will be used directly by the image generator tool if media_type is 'image'.")
    
    video_script: Optional[str] = Field(None, description="A script or key messages for video content if media_type is 'video'.")
    video_style: Optional[str] = Field(None, description="Visual style for the video (e.g., 'animated infographic', 'product showcase', 'slideshow with text overlays') if media_type is 'video'.")
    video_duration_seconds: Optional[int] = Field(None, description="Desired duration of the video in seconds if media_type is 'video'.")
    video_assets_description: Optional[str] = Field(None, description="Description of assets needed for the video (e.g., 'stock footage of happy dogs', 'product shots') if media_type is 'video'.")

    color_palette: List[str] = Field(..., description="List of HEX or named colors to be used in the design (e.g., ['#FFFFFF', '#000000', 'red']).")
    typography: str = Field(..., description="Guidance on typography (e.g., 'Headline: Montserrat Bold, Body: Lato Regular').")
    layout_guidance: str = Field(..., description="Instructions on layout and composition (e.g., 'Single column, mobile-first, hero image top').")
    ml_reasoning: str = Field(..., description="ML-enhanced reasoning justifying the design choices.")
    proposed_design_change: Optional[str] = Field(None, description="A proposal for improving the design system, if applicable.")

# --- Example Usage (for testing/demonstration) ---
if __name__ == "__main__":
    try:
        design_spec = DesignSpecification(
            overall_visual_concept="Clean, minimalist, and approachable, emphasizing natural light.",
            image_prompt="A happy golden retriever puppy playing with a kitten in a sunlit modern kitchen, photorealistic.",
            color_palette=["#F5F1E9", "#4A5C3D", "#D4A24E", "#333333"],
            typography="Headlines: Montserrat Bold, Body: Lora Regular",
            layout_guidance="Mobile-optimized, hero image at top, clear sections for text.",
            ml_reasoning="Choices align with brand guidelines for warmth and trustworthiness, and optimize for mobile readability.",
            proposed_design_change="Standardize spacing units across all digital assets."
        )
        print("Valid Design Specification:")
        print(design_spec.json(indent=2))
    except Exception as e:
        print("Invalid Design Specification:", e)

    try:
        invalid_design_spec = DesignSpecification(
            overall_visual_concept="Missing image prompt",
            color_palette=["#000000"],
            typography="Arial",
            layout_guidance="Simple",
            ml_reasoning="Because it looks good."
        )
        print("Invalid Design Specification (expected error):")
        print(invalid_design_spec.json(indent=2))
    except Exception as e:
        print("Invalid Design Specification (expected error):", e)
