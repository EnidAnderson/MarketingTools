from pydantic import BaseModel, Field, HttpUrl, root_validator
from typing import List, Optional, Union, Literal # Added Literal
from datetime import datetime

# --- Base Social Media Post Model ---

class SocialMediaPostDraft(BaseModel):
    """
    Draft data structure for a social media post, used by text-generating agents.
    Does NOT enforce media content (image_paths or video_path) as it's a draft.
    """
    platform: str = Field(..., description="The social media platform (e.g., 'instagram', 'facebook').")
    text_content: str = Field(..., max_length=2200, description="The main text content/caption of the post.")
    image_paths: Optional[List[str]] = Field(None, description="List of paths to generated image files.")
    video_path: Optional[str] = Field(None, description="Path to a generated video file.")
    hashtags: List[str] = Field([], max_items=30, description="List of hashtags (max 30).")
    mentions: List[str] = Field([], description="List of user mentions (e.g., '@naturesdiet').")
    call_to_action_text: Optional[str] = Field(None, description="Call to action text (e.g., 'Shop Now', 'Learn More').")
    call_to_action_url: Optional[HttpUrl] = Field(None, description="URL for the call to action.")

class SocialMediaPost(BaseModel):
    """
    Final data structure for a social media post, containing common fields.
    Enforces that either an image or a video must be present.
    """
    platform: str = Field(..., description="The social media platform (e.g., 'instagram', 'facebook').")
    text_content: str = Field(..., max_length=2200, description="The main text content/caption of the post.")
    image_paths: Optional[List[str]] = Field(None, description="List of paths to generated image files.")
    video_path: Optional[str] = Field(None, description="Path to a generated video file.")
    hashtags: List[str] = Field([], max_items=30, description="List of hashtags (max 30).")
    mentions: List[str] = Field([], description="List of user mentions (e.g., '@naturesdiet').")
    call_to_action_text: Optional[str] = Field(None, description="Call to action text (e.g., 'Shop Now', 'Learn More').")
    call_to_action_url: Optional[HttpUrl] = Field(None, description="URL for the call to action.")
    
    @root_validator(pre=True, skip_on_failure=True)
    def check_media_content(cls, values):
        image_paths, video_path = values.get('image_paths'), values.get('video_path')
        if not image_paths and not video_path:
            raise ValueError("A social media post must have either an image or a video.")
        if image_paths and video_path:
            raise ValueError("A social media post cannot have both images and a video (choose one).")
        return values

# --- Platform-Specific Models ---

class InstagramPost(SocialMediaPost):
    """
    Data structure for an Instagram post.
    Inherits common fields and adds Instagram-specific ones.
    """
    platform: Literal["instagram"] = "instagram" # Enforce platform type with Literal
    text_content: str = Field(..., max_length=2200, description="Instagram caption (max 2200 characters).")
    location_tag: Optional[str] = Field(None, description="Optional location tag for the post.")
    alt_text: Optional[str] = Field(None, max_length=1000, description="Alt text for images (max 1000 characters).")
    
    @root_validator(skip_on_failure=True)
    def check_instagram_media(cls, values):
        if not values.get('image_paths') and not values.get('video_path'):
            raise ValueError("Instagram posts must have either images or a video.")
        if values.get('image_paths') and len(values['image_paths']) > 10:
            raise ValueError("Instagram posts can have a maximum of 10 images in a carousel.")
        return values

class FacebookPost(SocialMediaPost):
    """
    Data structure for a Facebook post.
    Inherits common fields and adds Facebook-specific ones.
    """
    platform: Literal["facebook"] = "facebook" # Enforce platform type with Literal
    text_content: str = Field(..., max_length=63206, description="Facebook post text (max 63206 characters).") # Effectively unlimited for practical purposes
    link_preview_url: Optional[HttpUrl] = Field(None, description="URL to generate a link preview.")
    scheduled_publish_time: Optional[datetime] = Field(None, description="Optional scheduled publish time for the post.")

# --- Union Type for Campaigns ---

class SocialMediaCampaignPost(BaseModel):
    """
    A union type that can hold a post for any supported social media platform.
    """
    post_data: Union[InstagramPost, FacebookPost]

# --- Example Usage (for testing/demonstration) ---
if __name__ == "__main__":
    # Example Instagram Post
    try:
        insta_post = InstagramPost(
            text_content="Check out our new Simply Raw for beginners! #rawfeeding #newpetowner",
            image_paths=["/path/to/img1.png"],
            hashtags=["rawfeeding", "newpetowner", "dogfood"],
            mentions=["@naturesdietofficial"],
            location_tag="Pet Paradise Store",
            alt_text="A happy dog eating Simply Raw food.",
        )
        print("Valid Instagram Post:", insta_post.dict())
    except ValueError as e:
        print("Invalid Instagram Post:", e)

    # Example Facebook Post
    try:
        fb_post = FacebookPost(
            text_content="Introducing Simply Raw: The perfect start for your new furry friend! Learn more about raw feeding benefits.",
            image_paths=["/path/to/fb_img.jpg"],
            hashtags=["SimplyRaw", "PetNutrition"],
            call_to_action_text="Learn More",
            call_to_action_url="https://www.naturesdiet.com/simplyraw",
            link_preview_url="https://www.naturesdiet.com/simplyraw",
            scheduled_publish_time=datetime(2026, 2, 1, 10, 0, 0),
        )
        print("Valid Facebook Post:", fb_post.dict())
    except ValueError as e:
        print("Invalid Facebook Post:", e)

    # Example of invalid post (no media)
    try:
        invalid_post = SocialMediaPost(
            platform="generic",
            text_content="This post has no media."
        )
        print("Invalid Generic Post:", invalid_post.dict())
    except ValueError as e:
        print("Invalid Generic Post (expected error):", e)
    
    # Example of invalid Instagram post (too many images)
    try:
        invalid_insta_post = InstagramPost(
            text_content="Too many images for Instagram!",
            image_paths=[f"/path/to/img{i}.png" for i in range(12)],
        )
        print("Invalid Instagram Post (expected error):", invalid_insta_post.dict())
    except ValueError as e:
        print("Invalid Instagram Post (too many images):", e)
