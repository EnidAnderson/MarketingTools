// rustBotNetwork/cli/src/main.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use app_core::data_models::persona::{Persona, PersonaModules, IdentityModule, PersonalityModule, AbilitiesModule, InteractionModule};
use app_core::persona_loader;
use app_core::llm_client; // Added llm_client import
use serde_json;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Persona related commands
    Persona {
        #[clap(subcommand)]
        command: PersonaCommands,
    },
}

#[derive(Subcommand, Debug)]
enum PersonaCommands {
    /// Creates a new persona
    Create {
        /// Name of the persona to create
        #[clap(short, long)]
        name: String,
    },
    /// Validates an existing persona
    Validate {
        /// Name of the persona to validate
        #[clap(short, long)]
        name: String,
    },
    /// Runs a campaign using the specified persona
    Run {
        /// Name of the persona to use for the campaign
        #[clap(short, long)]
        name: String,
        /// Objective for the campaign
        #[clap(short, long)]
        objective: String,
    },
    /// Lists all available personas
    List,
    /// Shows the raw JSON content of a persona
    Show {
        /// Name of the persona to show
        #[clap(short, long)]
        name: String,
    },
}

#[tokio::main] // Make main function async
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Persona { command } => match command {
            PersonaCommands::Create { name } => {
                println!("Creating persona: {}", name);
                create_persona(name)?;
            },
            PersonaCommands::Validate { name } => {
                println!("Validating persona: {}", name);
                validate_persona(name)?;
            },
            PersonaCommands::Run { name, objective } => {
                println!("Running campaign with persona '{}' for objective: '{}'", name, objective);
                run_campaign_with_persona(name, objective).await?; // Await the async function
            },
            PersonaCommands::List => {
                println!("Listing all available personas:");
                list_personas()?;
            },
            PersonaCommands::Show { name } => {
                println!("Showing persona: {}", name);
                show_persona(name)?;
            }
        },
    }
    Ok(())
}

// ... existing create_persona, validate_persona, list_personas, show_persona functions

fn create_persona(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let persona_dir = PathBuf::from("teams").join(name);
    fs::create_dir_all(&persona_dir)?;

    let persona_json_path = persona_dir.join("persona.json");
    let prompt_md_path = persona_dir.join("prompt.md");

    // Create a default persona
    let default_persona = Persona {
        persona_name: name.to_string(),
        description: format!("Default description for {} persona.", name),
        modules: PersonaModules {
            identity: IdentityModule {
                role_identity: "Default Role Identity.".to_string(),
                alias: format!("{} Alias", name),
            },
            personality: PersonalityModule {
                complementary_personality: "Default Complementary Personality.".to_string(),
                tone: "Default Tone.".to_string(),
            },
            abilities: AbilitiesModule {
                authority: vec!["Default Authority.".to_string()],
                non_authority: vec!["Default Non-Authority.".to_string()],
                quality_bar: vec!["Default Quality Bar.".to_string()],
            },
            interaction: InteractionModule {
                inputs: vec!["Default Input.".to_string()],
                outputs: vec!["Default Output.".to_string()],
                required_output_format: vec!["Default Output Format.".to_string()],
            },
            tools: vec![], // No tools by default
        },
        version: "1.0".to_string(),
    };

    let json_content = serde_json::to_string_pretty(&default_persona)?;
    fs::write(&persona_json_path, json_content)?;

    fs::write(&prompt_md_path, "# Default Prompt

This is the default prompt for your persona.")?;

    println!("Persona '{}' created successfully at {:?}", name, persona_dir);

    Ok(())
}

fn validate_persona(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let persona_json_path = PathBuf::from("teams").join(name).join("persona.json");

    if !persona_json_path.exists() {
        return Err(format!("Persona JSON file not found: {:?}", persona_json_path).into());
    }

    let json_content = fs::read_to_string(&persona_json_path)?;
    
    match serde_json::from_str::<Persona>(&json_content) {
        Ok(persona) => {
            println!("Persona '{}' is VALID. Details: {:?}", name, persona.persona_name);
            // Optionally, print more details or perform deeper semantic validation here
        },
        Err(e) => {
            return Err(format!("Persona '{}' is INVALID. Error: {}", name, e).into());
        }
    }

    Ok(())
}

async fn run_campaign_with_persona( // Made async
    name: &str,
    objective: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (persona, prompt_content) = persona_loader::load_persona(name).map_err(|e| e as Box<dyn std::error::Error>)?;

    println!("\n--- Loaded Persona Details ---");
    println!("Persona Name: {}", persona.persona_name);
    println!("Description: {}", persona.description);
    println!("Version: {}", persona.version);
    println!("Identity Role: {}", persona.modules.identity.role_identity);
    println!("Personality Tone: {}", persona.modules.personality.tone);
    println!("Tools: {:?}", persona.modules.tools);

    println!("\n--- Loaded Prompt Content ---");
    println!("{}", prompt_content);

    // Construct the full prompt for the LLM
    let full_llm_prompt = format!(
        "You are {}. Your role is '{}'. Your personality is '{}' with a '{}' tone. You are tasked with the following objective: '{}'.\n\nHere is additional context or instructions:\n{}\n\nYour response should be based on these guidelines.",
        persona.persona_name,
        persona.modules.identity.role_identity,
        persona.modules.personality.complementary_personality,
        persona.modules.personality.tone,
        objective,
        prompt_content
    );

    println!("\n--- Sending to LLM ---");
    let llm_response = llm_client::send_text_prompt(&full_llm_prompt).await.map_err(|e| e as Box<dyn std::error::Error>)?;

    println!("\n--- LLM Response ---");
    println!("{}", llm_response);

    println!("\nCampaign for objective '{}' initiated with persona '{}'. (Further campaign logic to be implemented here)", objective, name);

    Ok(())
}

fn list_personas() -> Result<(), Box<dyn std::error::Error>> {
    let teams_dir = PathBuf::from("teams");

    if !teams_dir.exists() || !teams_dir.is_dir() {
        println!("No 'teams' directory found or it's not a directory.");
        return Ok(());
    }

    let mut personas = Vec::new();
    for entry in fs::read_dir(&teams_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let persona_name = path.file_name().unwrap().to_string_lossy().to_string();
            let persona_json_path = path.join("persona.json");
            if persona_json_path.exists() {
                personas.push(persona_name);
            }
        }
    }

    if personas.is_empty() {
        println!("  No personas found.");
    } else {
        for persona in personas {
            println!("  - {}", persona);
        }
    }

    Ok(())
}

fn show_persona(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let persona_json_path = PathBuf::from("teams").join(name).join("persona.json");

    if !persona_json_path.exists() {
        return Err(format!("Persona JSON file not found: {:?}", persona_json_path).into());
    }

    let json_content = fs::read_to_string(&persona_json_path)?;
    println!("{}", json_content);

    Ok(())
}
