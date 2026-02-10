import os
import stability_sdk.interfaces.gooseai.generation.generation_pb2 as generation
from stability_sdk import client
from dotenv import load_dotenv
from typing import List, Union, Optional, Any # Added imports
from . import generation_budget_manager
from .generation_budget_manager import API_COSTS
from src.config import PROJECT_ROOT
from PIL import Image, ImageEnhance, ImageFilter, ImageDraw
from PIL.Image import Image as PILImage # Added import for PIL Image type
import io
import numpy as np
from datetime import datetime

from ..agents.photographic_stylist import PhotographicStylist

# NEW Gemini Imports
from google import genai
# from google.genai import Image as GenaiImage # REMOVED: Problematic import
from io import BytesIO

load_dotenv()

# Ensure STABILITY_API_KEY is set as an environment variable
if not os.getenv("STABILITY_API_KEY"):
    print("WARNING: STABILITY_API_KEY environment variable not set. Stability AI image generation will be skipped.")

stability_api = None
if os.getenv("STABILITY_API_KEY"):
    stability_api = client.StabilityInference(
        key=os.environ['STABILITY_API_KEY'],
        verbose=True,
        engine="stable-diffusion-xl-1024-v1-0",
    )

# NEW: Instantiate Photographic Stylist
photographic_stylist = PhotographicStylist()

def humanize_image(image_binary_data: bytes) -> bytes:
    """
    Applies 'humanizing' effects to an AI-generated image to reduce the artificial look.
    This includes adding subtle noise, slight sharpness, and contrast adjustments.
    """
    print("Applying humanization effects to the image...")
    img: PILImage = Image.open(io.BytesIO(image_binary_data)).convert("RGB")

    # 1. Add subtle Gaussian noise
    img_np: np.ndarray = np.array(img)
    mean: int = 0
    std_dev: int = 2
    noise: np.ndarray = np.random.normal(mean, std_dev, img_np.shape).astype('int')
    img_np = np.clip(img_np + noise, 0, 255).astype('uint8')
    img = Image.fromarray(img_np) # type: PILImage

    # 2. Slight sharpness adjustment
    enhancer_sharpness = ImageEnhance.Sharpness(img)
    img = enhancer_sharpness.enhance(1.05) # type: PILImage

    # 3. Slight contrast adjustment
    enhancer_contrast = ImageEnhance.Contrast(img)
    img = enhancer_contrast.enhance(1.02) # type: PILImage

    # 4. Subtle Chromatic Aberration
    img_np_ca: np.ndarray = np.array(img)
    r_ca, g_ca, b_ca = img_np_ca[:,:,0], img_np_ca[:,:,1], img_np_ca[:,:,2]

    shift_amount: int = 1
    r_ca_shifted: np.ndarray = np.roll(r_ca, shift=shift_amount, axis=0)
    r_ca_shifted = np.roll(r_ca_shifted, shift=shift_amount, axis=1)

    b_ca_shifted: np.ndarray = np.roll(b_ca, shift=-shift_amount, axis=0)
    b_ca_shifted = np.roll(b_ca_shifted, shift=-shift_amount, axis=1)

    img_np_ca_merged: np.ndarray = np.stack((r_ca_shifted, g_ca, b_ca_shifted), axis=-1)
    img = Image.fromarray(img_np_ca_merged) # type: PILImage

    # 5. Add Vignette Effect
    width, height = img.size
    Y, X = np.ogrid[0:height, 0:width]
    center_x, center_y = width / 2, height / 2
    max_dist = np.sqrt(center_x**2 + center_y**2)
    
    dist_from_center = np.sqrt((X - center_x)**2 + (Y - center_y)**2)
    alpha_mask = 1 - (dist_from_center / max_dist * 0.2)
    alpha_mask = np.clip(alpha_mask + 0.3, 0, 1)

    alpha_mask_pil: PILImage = Image.fromarray((alpha_mask * 255).astype(np.uint8), mode='L')
    
    black_img: PILImage = Image.new('RGB', (width, height), (0, 0, 0))
    black_img.putalpha(alpha_mask_pil)

    img = Image.composite(img, black_img, black_img.getchannel('A')) # type: PILImage

    output_buffer: io.BytesIO = io.BytesIO()
    img.save(output_buffer, format="PNG")
    print("Humanization effects applied.")
    return output_buffer.getvalue()

def generate_image_with_gemini(prompt: str, campaign_dir: str, reference_image_path: Optional[str] = None) -> Optional[str]:
    """
    Generates an image using Google Gemini's image generation capabilities,
    optionally using a reference image.
    """
    api_key: Optional[str] = os.getenv("GEMINI_API_KEY") or os.getenv("GOOGLE_API_KEY")
    if not api_key:
        print("Gemini image generation failed: GEMINI_API_KEY or GOOGLE_API_KEY environment variable not set.")
        return "Gemini image generation failed: API key not set."

    print("--- Gemini Image Generation Request ---")

    cost: float = API_COSTS["gemini-image-generation"]["per_image"]
    
    if not generation_budget_manager.can_generate(cost):
        return "Gemini image generation failed: Budget limit reached."

    print(f"This Gemini image generation will cost ${cost:.4f}.")
    
    try:
        client: genai.Client = genai.Client(api_key=api_key)
        
        # Prepare the prompt content
        content: List[Union[str, PILImage]] = [prompt] # Use PILImage for type hinting
        if reference_image_path:
            print(f"Using reference image: {reference_image_path}")
            try:
                img_ref: PILImage = Image.open(reference_image_path)
                content.insert(0, genai.GenerativeModel.from_pil(img_ref)) # Convert PILImage to GenaiImage
            except FileNotFoundError:
                print(f"Error: Reference image not found at {reference_image_path}")
                return f"Gemini image generation failed: Reference image not found."
            except Exception as e:
                print(f"Error opening reference image: {e}")
                return f"Gemini image generation failed: Could not open reference image."

        response: Any = client.models.generate_content(
            model="models/nano-banana-pro-preview",
            contents=content,
        )

        # Ensure response has parts before iterating
        if hasattr(response, 'parts') and response.parts:
            for part in response.parts:
                if hasattr(part, 'inline_data') and part.inline_data:
                    image: Optional[PILImage] = part.as_image()
                    if image: # Ensure image is not None
                        filename_base: str = prompt[:50].replace(' ', '_').replace('/', '_')
                        timestamp: str = datetime.now().strftime("%Y%m%d%H%M%S")
                        img_path: str = os.path.join(campaign_dir, f"{filename_base}_{timestamp}_gemini.png")
                        os.makedirs(campaign_dir, exist_ok=True)
                        image.save(img_path)
                        generation_budget_manager.record_generation(cost, "gemini-image-generation")
                        print(f"Gemini image saved to {img_path}")
                        return img_path

        print("No image data found in Gemini response.")
        # --- DEBUG START ---
        print(f"Full Gemini response object: {response}")
        print(f"Full Gemini response parts: {response.parts}")
        # --- DEBUG END ---
        return "Gemini image generation failed: No image data."

    except Exception as e:
        print(f"Gemini image generation failed: {e}")
        return f"Gemini image generation failed: {e}"

def generate_image(prompt: str, campaign_dir: str, reference_image_path: Optional[str] = None) -> Optional[str]:
    """
    Generates an image using the Stability AI API, with budget checks,
    applies humanization effects, and now uses a Photographic Stylist
    to enrich the prompt. Falls back to Gemini image generation if Stability AI fails.
    An optional reference image can be passed for image-to-image generation with Gemini.
    """
    print("--- Image Generation Request ---")
    
    stability_cost: float = API_COSTS["stable-diffusion-xl-1024-v1-0"]["per_image"]

    # If a reference image is provided, skip Stability and go directly to Gemini
    if reference_image_path:
        print("Reference image provided. Skipping Stability AI and using Gemini for image-to-image generation.")
        return generate_image_with_gemini(prompt, campaign_dir, reference_image_path)
    
    # Try Stability AI first (only if no reference image)
    if stability_api and os.getenv("STABILITY_API_KEY") and generation_budget_manager.can_generate(stability_cost):
        print(f"Attempting Stability AI. This image generation will cost ${stability_cost:.4f}.")

        # NEW: Enrich the prompt using the Photographic Stylist
        base_prompt: str = prompt
        print(f"Original prompt: {base_prompt}")
        enriched_prompt: str = photographic_stylist.enrich_prompt(base_prompt)
        print(f"Enriched_prompt: {enriched_prompt}")

        print(f"Generating image with prompt: {enriched_prompt}")
        
        try:
            answers: Any = stability_api.generate(
                prompt=enriched_prompt,
                seed=42,
                steps=50,
                cfg_scale=8.0,
                width=1024,
                height=1024,
                samples=1,
                sampler=generation.SAMPLER_K_DPMPP_2M
            )
            
            for resp in answers:
                for artifact in resp.artifacts:
                    if artifact.type == generation.ARTIFACT_IMAGE:
                        humanized_image_binary: bytes = humanize_image(artifact.binary)
                        filename_base: str = enriched_prompt[:50].replace(' ', '_').replace('/', '_')
                        timestamp: str = datetime.now().strftime("%Y%m%d%H%M%S")
                        img_path: str = os.path.join(campaign_dir, f"{filename_base}_{timestamp}_humanized.png")
                        os.makedirs(campaign_dir, exist_ok=True)
                        with open(img_path, "wb") as f:
                            f.write(humanized_image_binary)
                        
                        generation_budget_manager.record_generation(stability_cost, "stability-sdk")
                        print(f"Humanized image saved to {img_path}")
                        return img_path
            print("No image data found from Stability AI.")
        except client.grpc._channel._MultiThreadedRendezvous as e: # Catch specific gRPC error
            print(f"Stability AI image generation failed due to RESOURCE_EXHAUSTED: {e}. Falling back to Gemini.")
            return generate_image_with_gemini(prompt, campaign_dir, reference_image_path)
        except Exception as e:
            print(f"Stability AI image generation failed: {e}. Falling back to Gemini.")
            return generate_image_with_gemini(prompt, campaign_dir, reference_image_path)
    else:
        print("Stability AI not available (API key not set or budget exceeded, or client not initialized). Falling back to Gemini.")
        return generate_image_with_gemini(prompt, campaign_dir, reference_image_path)
    
    return "Image generation failed." # Default return if all attempts fail