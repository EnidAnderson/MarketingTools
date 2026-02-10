# Tauri App Rust Tool Integration & Pipeline Development Plan

## Status

This document remains useful as historical execution context, but active planning is now tracked in:

1. `/Volumes/EnidsAssets/NaturesDietMarketingTeam/MEMORY.md`
2. `/Volumes/EnidsAssets/NaturesDietMarketingTeam/planning/AGENT_PLATFORM_ARCHITECTURE.md`
3. `/Volumes/EnidsAssets/NaturesDietMarketingTeam/planning/TAURI_ASYNC_BRIDGE_STRATEGY.md`
4. `/Volumes/EnidsAssets/NaturesDietMarketingTeam/planning/RUST_MIGRATION_MASTER_PLAN.md`
5. `/Volumes/EnidsAssets/NaturesDietMarketingTeam/planning/IMPLEMENTATION_BACKLOG.md`

This document outlines the phased plan to transition all marketing tools to Rust, integrate them into the Tauri desktop application, and build user-friendly marketing pipelines.

## Hardening standards (mandatory)

1. `planning/BUDGET_GUARDRAILS_STANDARD.md` defines cost controls and stop behavior.
2. `planning/RELEASE_GATES_POLICY.md` defines non-negotiable publish gates.
3. `planning/ADR_TRIGGER_RULES.md` defines ADR requirements for architecture-impacting change.
4. `planning/SECURITY_THREAT_MODEL.md` and `planning/SECURITY_CONTROL_BASELINE.md` define risk/control ownership.
5. `planning/BUDGET_ENVELOPE_SCHEMA.md` defines run-level envelope fields and validation rules.
6. `planning/ROLE_PERMISSION_MATRIX.md` defines least-privilege role permissions for critical actions.
7. `planning/EXTERNAL_PUBLISH_CONTROL.md` defines two-person external publish control and rollback protocol.

Any future planning items must preserve these controls and cannot bypass them.

---

### Phase I: Port Core Creative Tools to Rust

**Goal:** Replace the Python-based image and video generation/manipulation tools with high-priority Rust equivalents integrated into the UI.

*   [x] **1. Port `image_generation.py` to Rust**
    *   [x] Analyze Python tool's functionality (Gemini/Stability AI, prompt enrichment, file saving).
    *   [x] Identify and add necessary Rust crates (`dotenv`, `reqwest`, `serde_json`, `image`, `base64`).
    *   [x] Implement `generate_image` function in `app_core/src/image_generator.rs` to call the Gemini API.
    *   [x] Create Tauri command `generate_image_command` in `src-tauri/src/lib.rs`.
    *   [x] Integrate the `generate_image` tool into the frontend UI in `frontend/main.js`.
    *   [ ] Enhance `generate_image` in Rust to support image-to-image generation (using a reference image).
    *   [ ] Port the "humanizing" effects (noise, contrast, etc.) from Python to a new Rust function using the `image` crate.

*   [x] **2. Port `screenshot_tool.py` to Rust**
    *   [x] Analyze Python tool's functionality.
    *   [x] Confirm Rust command `screenshot` in `src-tauri/src/lib.rs`.
    *   [ ] Integrate the `screenshot` tool into the frontend UI. (Assumed not yet done, as UI is not generic)

*   [x] **3. Port `css_analyzer.py` to Rust**
    *   [x] Analyze Python tool's functionality.
    *   [x] Confirm Rust command `analyze_css` in `src-tauri/src/lib.rs`.
    *   [ ] Integrate the `analyze_css` tool into the frontend UI.

*   [x] **4. Port `html_bundler.py` to Rust**
    *   [x] Analyze Python tool's functionality.
    *   [x] Confirm Rust command `bundle_html` in `src-tauri/src/lib.rs`.
    *   [ ] Integrate the `bundle_html` tool into the frontend UI.

*   [ ] **5. Port `video_generator_tool.py` to Rust**
    *   [ ] Analyze Python tool's functionality (likely uses an external API like Stable Diffusion Video or a sequence-to-video library).
    *   [ ] Research and select a Rust crate or REST API client for a video generation service.
    *   [ ] Implement a `generate_video` function in a new `app_core/src/video_generator.rs` module.
    *   [ ] Create a Tauri command `generate_video_command` in `src-tauri/src/lib.rs`.
    *   [ ] Add the `generate_video` tool to the `frontend/main.js` UI, including relevant inputs (e.g., prompt, image paths, duration).

*   [ ] **6. Port `image_manipulation_tool.py` to Rust**
    *   [ ] Analyze Python tool's functionality (e.g., resize, add watermark using Pillow).
    *   [ ] Implement `resize_image` and `add_watermark` functions in a new `app_core/src/image_manipulator.rs` module using the `image` crate.
    *   [ ] Create Tauri commands (`resize_image_command`, `add_watermark_command`) in `src-tauri/src/lib.rs`.
    *   [ ] Add these tools to the frontend UI, including file selectors for input/output paths and inputs for parameters (e.g., dimensions, watermark text).

*   [ ] **7. Port `gif_generator_tool.py` to Rust**
    *   [ ] Analyze Python tool's functionality (likely uses `ffmpeg`).
    *   [ ] Implement a Rust function in a new `app_core/src/gif_generator.rs` module that calls the `ffmpeg` command-line tool using `std::process::Command`.
    *   [ ] Create a Tauri command `generate_gif_command` in `src-tauri/src/lib.rs`.
    *   [ ] Add the `generate_gif` tool to the frontend UI, with inputs for video path, output path, and timing parameters).

---

### Phase II: Frontend UI & UX Enhancements

**Goal:** Make the application robust and user-friendly for non-technical marketers.

*   [x] **1. Implement Dynamic Tool UI Generation**
    *   [x] Replace text input for `campaign_dir` with a "Choose Directory" button.
    *   [x] Implement file dialog using `window.__TAURI__.dialog.open`.
    *   [ ] Create a generic function in `main.js` to render different input types based on tool parameter definitions (e.g., `type: 'file'`, `type: 'string'`, `enum: ['option1', 'option2']`).
    *   [ ] Add a file picker for the `reference_image_path` parameter in the `generate_image` tool.
    *   [ ] Implement a loading indicator in the UI while a tool is executing.
    *   [ ] Display generated images directly in the UI instead of just showing the file path.

*   [x] **2. Create a Central Rust Tool Registry**
    *   [x] Create a `Tool` trait and `ToolDefinition` struct in `app_core`. (Implicitly exists via `tool_registry` module).
    *   [x] Implement a `ToolRegistry` in `app_core` to hold all available Rust tools. (Confirmed by `tool_registry` module).
    *   [x] Create a Tauri command `get_rust_tools` that returns a `Vec<ToolDefinition>` to the frontend. (Confirmed `get_tools` command).
    *   [ ] Modify `frontend/main.js` to fetch this list dynamically instead of using a hardcoded `availableTools` array.

---

### Phase III: Marketing Pipeline Implementation

**Goal:** Allow marketers to chain tools together to create full campaigns.

*   [ ] **1. Design Pipeline Structure**
    *   [ ] Define a data structure in Rust (e.g., `Pipeline` struct) that represents a sequence of tool calls with their parameters.
    *   [ ] Design a mechanism for passing the output of one tool as the input to the next (e.g., an execution context or state object).

*   [ ] **2. Implement Pipeline Execution in Rust**
    *   [ ] Create a `run_pipeline` function in `app_core` that iterates through a `Pipeline`'s steps and executes the corresponding tools.
    *   [ ] Create a Tauri command `run_pipeline_command` that takes a `Pipeline` object as input.

*   [ ] **3. Design and Implement Pipeline UI**
    *   [ ] Create a new "Pipelines" tab or section in the frontend UI.
    *   [ ] Design a drag-and-drop or step-by-step interface for users to build a pipeline by selecting and configuring tools.
    *   [ ] Implement saving and loading of user-created pipelines.
    *   [ ] Add a "Run Pipeline" button and a comprehensive output area to show the progress and final results of the entire pipeline.

---

### Phase IV: Cleanup and Finalization

**Goal:** Remove obsolete Python code and finalize the Rust-based application.

*   [ ] **1. Deprecate and Remove Python Tools**
    *   [ ] Once all high-priority tools are ported, remove the `python_runner.rs` module from `app_core`.
    *   [ ] Remove the `src/python_tool_dispatcher.py` file.
    *   [ ] Remove all Python tool files from `src/tools/`.
    *   [ ] Remove `tool_registry.py`.

*   [ ] **2. Final Code Review and Refactoring**
    *   [ ] Review all Rust code for consistency, error handling, and adherence to conventions.
    *   [ ] Remove unused warnings and dependencies (e.g., `cargo fix --lib -p app_core`).
    *   [ ] Add documentation to public functions in `app_core`.
    *   [ ] Ensure all user-facing errors are clear and actionable.
