// rustBotNetwork/app_core/src/persona_loader.rs

use crate::data_models::persona::Persona;
use std::path::{Path, PathBuf};
use std::fs;

/// Loads a persona from the 'teams' directory.
///
/// This function expects a directory structure like:
/// `teams/<persona_name>/persona.json`
/// `teams/<persona_name>/prompt.md`
pub fn load_persona(persona_name: &str) -> Result<(Persona, String), Box<dyn std::error::Error>> {
    let persona_dir = PathBuf::from("teams").join(persona_name);

    let persona_json_path = persona_dir.join("persona.json");
    if !persona_json_path.exists() {
        return Err(format!("Persona JSON file not found: {:?}", persona_json_path).into());
    }

    let prompt_md_path = persona_dir.join("prompt.md");
    if !prompt_md_path.exists() {
        return Err(format!("Prompt Markdown file not found: {:?}", prompt_md_path).into());
    }

    // Load persona.json
    let json_content = fs::read_to_string(&persona_json_path)?;
    let persona: Persona = serde_json::from_str(&json_content)?;

    // Load prompt.md
    let prompt_content = fs::read_to_string(&prompt_md_path)?;

    Ok((persona, prompt_content))
}
