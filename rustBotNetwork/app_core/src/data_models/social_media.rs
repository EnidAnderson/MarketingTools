use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SocialMediaPost {
    pub platform: String,
    pub text_content: String,
    pub image_paths: Option<Vec<String>>,
    pub video_path: Option<String>,
    pub hashtags: Vec<String>,
    pub mentions: Vec<String>,
    pub call_to_action_text: Option<String>,
    pub call_to_action_url: Option<String>, // Changed from Option<Url>
}
