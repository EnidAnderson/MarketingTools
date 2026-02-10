use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DesignSpecification {
    pub overall_visual_concept: String,
    pub media_type: String,
    pub image_prompt: Option<String>,
    pub video_script: Option<String>,
    pub video_style: Option<String>,
    pub video_duration_seconds: Option<i32>,
    pub video_assets_description: Option<String>,
    pub color_palette: Vec<String>,
    pub typography: String,
    pub layout_guidance: String,
    pub ml_reasoning: String,
    pub proposed_design_change: Option<String>,
}
