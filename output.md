I have completed the requested changes.

1.  **Nano Banana (Gemini Image Generation) Fallback:**
    *   I have implemented a fallback mechanism in `src/tools/image_generation.py`. If Stability AI fails (e.g., due to budget exhaustion), the system will now attempt to generate the image using Google Gemini's `gemini-2.5-flash-image` model.
    *   I've updated `src/data/api_costs.json` to include a placeholder cost for Gemini image generation.
    *   The latest test run successfully demonstrated this fallback, with Stability AI failing and Gemini generating an image.

2.  **Incremental Agent Output Saving:**
    *   I have modified `src/graph.py` to ensure that key agent outputs are saved incrementally as files within the campaign directory as they are generated. This provides partial results even if the campaign does not complete fully.
    *   Specifically, `campaign_goal.md`, `research_context.md`, `marketing_content_CURRENT.md`, and `design_specs_CURRENT.md` are now saved after their respective agents complete their tasks.

The system is now more resilient to external API failures and provides more immediate feedback on campaign progress.