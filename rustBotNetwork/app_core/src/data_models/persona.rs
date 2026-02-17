// rustBotNetwork/app_core/src/data_models/persona.rs

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Persona {
    pub persona_name: String,
    pub description: String,
    pub modules: PersonaModules,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonaModules {
    pub identity: IdentityModule,
    pub personality: PersonalityModule,
    pub abilities: AbilitiesModule,
    pub interaction: InteractionModule,
    pub tools: Vec<String>, // List of tool IDs or names
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityModule {
    pub role_identity: String,
    pub alias: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonalityModule {
    pub complementary_personality: String,
    pub tone: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AbilitiesModule {
    pub authority: Vec<String>,
    pub non_authority: Vec<String>,
    pub quality_bar: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InteractionModule {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub required_output_format: Vec<String>,
}
