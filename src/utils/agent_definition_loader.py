import os
from typing import Dict, List, Optional

# This would typically come from a config file or environment
ACTIVE_PROMPT_VERSIONS: Dict[str, int] = {
    "StrategistAgent": 1,
    "ResearcherAgent": 1,
    # ... other agents
}

class AgentDefinitionLoader:
    def __init__(self, base_dir: str = "agent_definitions"):
        self.base_dir = base_dir

    def _load_file_content(self, file_path: str) -> Optional[str]:
        """Loads content from a single file, returns None if not found."""
        if os.path.exists(file_path):
            with open(file_path, 'r') as f:
                return f.read().strip()
        return None

    def _load_module(self, agent_name: str, part_name: str, version: int) -> Optional[str]:
        """Loads a specific part of an agent's definition for a given version."""
        # Example path: agent_definitions/StrategistAgent/v1/identity.txt
        file_path = os.path.join(
            self.base_dir,
            agent_name,
            f"v{version}",
            f"{part_name}.txt"
        )
        return self._load_file_content(file_path)

    def get_full_agent_context(self, agent_name: str, include_few_shot_examples: bool = False) -> str:
        """
        Combines all modular parts of an agent's definition.
        Brand foundation is prioritized (loaded first).
        """
        version = ACTIVE_PROMPT_VERSIONS.get(agent_name, 1) # Default to v1
        context_parts: List[str] = []

        # 1. Brand Foundation (prioritized)
        brand_foundation = self._load_module("GLOBAL", "brand_foundation", 1) # Assume global brand foundation v1
        if brand_foundation:
            context_parts.append(brand_foundation)

        # 2. Agent Identity
        identity = self._load_module(agent_name, "identity", version)
        if identity:
            context_parts.append(identity)

        # 3. Agent Expertise
        expertise = self._load_module(agent_name, "expertise", version)
        if expertise:
            context_parts.append(expertise)

        # 4. Agent Goals
        goals = self._load_module(agent_name, "goals", version)
        if goals:
            context_parts.append(goals)

        # 5. Few-shot examples
        if include_few_shot_examples:
            few_shot = self._load_module(agent_name, "few_shot_examples", version)
            if few_shot:
                context_parts.append(few_shot)

        return "\n\n".join(context_parts).strip()

# Example Usage (requires agent_definitions directory structure to exist)
if __name__ == "__main__":
    # Create dummy agent definition files for testing
    os.makedirs("agent_definitions/GLOBAL/v1", exist_ok=True)
    with open("agent_definitions/GLOBAL/v1/brand_foundation.txt", "w") as f:
        f.write("Our brand believes in natural, healthy pet food.")

    os.makedirs("agent_definitions/StrategistAgent/v1", exist_ok=True)
    with open("agent_definitions/StrategistAgent/v1/identity.txt", "w") as f:
        f.write("You are an expert marketing strategist.")
    with open("agent_definitions/StrategistAgent/v1/expertise.txt", "w") as f:
        f.write("You are skilled in market analysis and campaign planning.")
    with open("agent_definitions/StrategistAgent/v1/goals.txt", "w") as f:
        f.write("Your goal is to create effective marketing campaign strategies.")
    with open("agent_definitions/StrategistAgent/v1/few_shot_examples.txt", "w") as f:
        f.write("Example 1: ...\nExample 2: ...")

    loader = AgentDefinitionLoader()
    strategist_context = loader.get_full_agent_context(
        "StrategistAgent",
        include_few_shot_examples=True
    )
    print("--- Strategist Agent Context ---")
    print(strategist_context)

    researcher_context = loader.get_full_agent_context("ResearcherAgent")
    print("\n--- Researcher Agent Context (minimal) ---")
    print(researcher_context) # Should be empty or only brand foundation if files don't exist

    # Clean up dummy files
    import shutil
    shutil.rmtree("agent_definitions")
