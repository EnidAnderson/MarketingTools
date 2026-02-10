use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// This would typically come from a config file or environment
// Using a Lazy static HashMap to store active prompt versions
static ACTIVE_PROMPT_VERSIONS: Lazy<HashMap<String, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("StrategistAgent".to_string(), 1);
    m.insert("ResearcherAgent".to_string(), 1);
    m // return the map
});

/// Loads agent definition components from the file system.
pub struct AgentDefinitionLoader {
    base_dir: PathBuf,
}

impl AgentDefinitionLoader {
    /// Creates a new `AgentDefinitionLoader` with a specified base directory.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Loads content from a single file, returns `None` if not found or cannot be read.
    fn _load_file_content(&self, file_path: &Path) -> Option<String> {
        fs::read_to_string(file_path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Loads a specific part of an agent's definition for a given version.
    /// Example path: agent_definitions/StrategistAgent/v1/identity.txt
    fn _load_module(&self, agent_name: &str, part_name: &str, version: u32) -> Option<String> {
        let file_path = self
            .base_dir
            .join(agent_name)
            .join(format!("v{}", version)) // Corrected format! macro
            .join(format!("{}.txt", part_name));
        self._load_file_content(&file_path)
    }

    /// Combines all modular parts of an agent's definition.
    /// Brand foundation is prioritized (loaded first).
    pub fn get_full_agent_context(
        &self,
        agent_name: &str,
        include_few_shot_examples: bool,
    ) -> String {
        let version = *ACTIVE_PROMPT_VERSIONS.get(agent_name).unwrap_or(&1); // Default to v1
        let mut context_parts: Vec<String> = Vec::new();

        // 1. Brand Foundation (prioritized)
        if let Some(bf) = self._load_module("GLOBAL", "brand_foundation", 1) {
            // Assume global brand foundation v1
            context_parts.push(bf);
        }

        // 2. Agent Identity
        if let Some(identity) = self._load_module(agent_name, "identity", version) {
            context_parts.push(identity);
        }

        // 3. Agent Expertise
        if let Some(expertise) = self._load_module(agent_name, "expertise", version) {
            context_parts.push(expertise);
        }

        // 4. Agent Goals
        if let Some(goals) = self._load_module(agent_name, "goals", version) {
            context_parts.push(goals);
        }

        // 5. Few-shot examples
        if include_few_shot_examples {
            if let Some(few_shot) = self._load_module(agent_name, "few_shot_examples", version) {
                context_parts.push(few_shot);
            }
        }

        context_parts.join("\n\n").trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    // Removed: use std::io::Write;

    // Helper to create dummy agent definition files
    fn create_dummy_agent_files(base_path: &Path) {
        // Global brand foundation
        fs::create_dir_all(base_path.join("GLOBAL/v1")).unwrap();
        fs::write(
            base_path.join("GLOBAL/v1/brand_foundation.txt"),
            "Our brand believes in natural, healthy pet food.",
        )
        .unwrap();

        // StrategistAgent v1
        fs::create_dir_all(base_path.join("StrategistAgent/v1")).unwrap();
        fs::write(
            base_path.join("StrategistAgent/v1/identity.txt"),
            "You are an expert marketing strategist.",
        )
        .unwrap();
        fs::write(
            base_path.join("StrategistAgent/v1/expertise.txt"),
            "You are skilled in market analysis and campaign planning.",
        )
        .unwrap();
        fs::write(
            base_path.join("StrategistAgent/v1/goals.txt"),
            "Your goal is to create effective marketing campaign strategies.",
        )
        .unwrap();
        fs::write(
            base_path.join("StrategistAgent/v1/few_shot_examples.txt"),
            "Example 1:\n- Create social media post\nExample 2:\n- Write blog draft",
        )
        .unwrap();

        // ResearcherAgent v1 (minimal)
        fs::create_dir_all(base_path.join("ResearcherAgent/v1")).unwrap();
        fs::write(
            base_path.join("ResearcherAgent/v1/identity.txt"),
            "You are a diligent researcher.",
        )
        .unwrap();
    }

    #[test]
    fn test_load_file_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\n").unwrap();

        let loader = AgentDefinitionLoader::new(dir.path());
        assert_eq!(
            loader._load_file_content(&file_path),
            Some("Hello World".to_string())
        );

        let non_existent_file = dir.path().join("non_existent.txt");
        assert_eq!(loader._load_file_content(&non_existent_file), None);
    }

    #[test]
    fn test_load_module() {
        let dir = tempdir().unwrap();
        create_dummy_agent_files(dir.path());

        let loader = AgentDefinitionLoader::new(dir.path());
        assert_eq!(
            loader._load_module("StrategistAgent", "identity", 1),
            Some("You are an expert marketing strategist.".to_string())
        );
        assert_eq!(
            loader._load_module("GLOBAL", "brand_foundation", 1),
            Some("Our brand believes in natural, healthy pet food.".to_string())
        );
        assert_eq!(loader._load_module("NonExistentAgent", "identity", 1), None);
        assert_eq!(
            loader._load_module("StrategistAgent", "non_existent_part", 1),
            None
        );
        assert_eq!(
            loader._load_module("StrategistAgent", "identity", 99), // Wrong version
            None
        );
    }

    #[test]
    fn test_get_full_agent_context_strategist() {
        let dir = tempdir().unwrap();
        create_dummy_agent_files(dir.path());

        let loader = AgentDefinitionLoader::new(dir.path());
        let context = loader.get_full_agent_context("StrategistAgent", true);

        let expected_context_parts = vec![
            "Our brand believes in natural, healthy pet food.",
            "You are an expert marketing strategist.",
            "You are skilled in market analysis and campaign planning.",
            "Your goal is to create effective marketing campaign strategies.",
            "Example 1:\n- Create social media post\nExample 2:\n- Write blog draft",
        ];
        let expected_context = expected_context_parts.join("\n\n");
        assert_eq!(context, expected_context);
    }

    #[test]
    fn test_get_full_agent_context_strategist_no_few_shot() {
        let dir = tempdir().unwrap();
        create_dummy_agent_files(dir.path());

        let loader = AgentDefinitionLoader::new(dir.path());
        let context = loader.get_full_agent_context("StrategistAgent", false);

        let expected_context_parts = vec![
            "Our brand believes in natural, healthy pet food.",
            "You are an expert marketing strategist.",
            "You are skilled in market analysis and campaign planning.",
            "Your goal is to create effective marketing campaign strategies.",
        ];
        let expected_context = expected_context_parts.join("\n\n");
        assert_eq!(context, expected_context);
    }

    #[test]
    fn test_get_full_agent_context_researcher() {
        let dir = tempdir().unwrap();
        create_dummy_agent_files(dir.path());

        let loader = AgentDefinitionLoader::new(dir.path());
        let context = loader.get_full_agent_context("ResearcherAgent", false);

        let expected_context_parts = vec![
            "Our brand believes in natural, healthy pet food.",
            "You are a diligent researcher.",
        ];
        let expected_context = expected_context_parts.join("\n\n");
        assert_eq!(context, expected_context);
    }

    #[test]
    fn test_get_full_agent_context_missing_agent() {
        let dir = tempdir().unwrap();
        create_dummy_agent_files(dir.path());

        let loader = AgentDefinitionLoader::new(dir.path());
        let context = loader.get_full_agent_context("NonExistentAgent", true);
        // Only brand foundation should be loaded if it exists
        assert_eq!(
            context,
            "Our brand believes in natural, healthy pet food.".to_string()
        );
    }

    #[test]
    fn test_empty_base_dir() {
        let dir = tempdir().unwrap();
        let loader = AgentDefinitionLoader::new(dir.path());
        let context = loader.get_full_agent_context("StrategistAgent", true);
        assert_eq!(context, ""); // Should be empty as no files exist
    }
}
