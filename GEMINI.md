
---

### **Comprehensive Testing Plan Suite (Formatted Checklist)**

The following plan outlines the types of tests, what needs to be mocked, and key scenarios for each major component and workflow. The goal is to ensure functional correctness and fault tolerance **without incurring any API costs** or relying on live external services, achieved through extensive mocking and dummy data.

#### **I. General Testing Principles & Mocking Strategy**

*   [ ] All calls to `langchain_google_genai.ChatGoogleGenerativeAI` (`llm.invoke`) will be mocked to return predefined strings (e.g., valid Markdown, JSON tool calls, error messages).
*   [ ] All network requests (`requests.get`, `requests.post`) will be mocked using `unittest.mock.patch` or `pytest-mock`.
*   [ ] All `subprocess.run` calls will be mocked to return predefined `stdout`/`stderr` and `returncode`.
*   [ ] All `chromadb` client interactions (`client.get_or_create_collection`, `collection.add`, `collection.query`) will be mocked.
*   [ ] File System Interaction: For reading/writing configuration/definition files, mock file I/O or use `pytest`'s `tmp_path`. For temporary image/report files, use `tmp_path`.
*   [ ] Environment Variables: Use `patch.dict(os.environ, ...)` to control tool availability checks (`is_available()`).
*   [ ] Controlled State: For LangGraph workflows, ensure the initial `AgentState` is well-defined.

#### **II. Core Components Unit Test Plan**

1.  **`src/utils/logger.py`:**
    *   [x] Verify `JsonFormatter` outputs valid JSON with expected fields (`timestamp`, `level`, `message`, `campaign_id`).
    *   [x] Verify `set_current_campaign_id` correctly injects `campaign_id` into log records.
    *   [x] Verify `log_agent_event`, `log_llm_call`, `log_tool_use` generate correctly structured log entries.

2.  **`src/utils/scheduled_task_registry.py`:**
    *   [x] `add_schedule`: Verify schedule is correctly added and saved to file.
    *   [x] `get_schedules`: Verify retrieval, filtering by frequency, and `enabled` status.
    *   [x] `update_last_run`: Verify timestamp is updated.
    *   [x] `remove_schedule`: Verify removal.

3.  **`src/utils/agent_definition_loader.py`:**
    *   [x] `_load_module`: Verify it loads content from the correct versioned file based on `ACTIVE_PROMPT_VERSIONS`.
    *   [x] `get_full_agent_context`: Verify it combines all modular parts (expertise, identity, goals, etc.) in the correct order, with brand foundation prioritized, and includes few-shot examples when requested.
    *   [x] Verify graceful handling of missing definition files.

4.  **`src/tools/base_tool.py` (Abstract Class):**
    *   [x] (No direct tests for abstract class, but its implementation in concrete tools will be tested.)

5.  **`src/tools/tool_registry.py`:**
    *   [x] `register_tool`: Verify tools are added to the internal dictionary.
    *   [x] `get_tool_instance`: Verify it returns an instantiated tool if available, `None` if not found or `is_available()` returns `False`.
    *   [x] `get_available_tool_descriptions`: Verify it returns correct JSON schema-like descriptions only for available tools.

#### **III. Tools Unit Test Plan**

1.  **`src/tools/email_sender_tool.py`:**
    *   [x] `is_available()`: Returns `True` if all SMTP env vars are set, `False` otherwise.
    *   [x] `run()` (success): Mocks `server.send_message`, asserts it's called with correct `MIMEText`.
    *   [x] `run()` (failure): Mocks `server.send_message` to raise exception, asserts `status: error`.

2.  **`src/tools/event_calendar_tool.py`:**
    *   [x] `is_available()`: Always `True` (conceptual mock tool).
    *   [x] `run()`: Returns correct mock events for given date range.
    *   [x] `run()` with `keywords`: Filters events correctly.

3.  **`src/tools/human_feedback_tool.py`:**
    *   [x] `collect_feedback`: Verifies `MemoryRetrievalTool.store_memory` is called with correct JSON document and metadata.
    *   [x] `retrieve_relevant_feedback`: Verifies `MemoryRetrievalTool.retrieve_memory` is called and returned JSON is parsed.

4.  **`src/tools/image_manipulation_tool.py`:**
    *   [x] `is_available()`: Returns `True` if `magick -version` succeeds, `False` otherwise.
    *   [x] `run()` (`resize`): Verifies correct `magick` command is formed and executed.
    *   [x] `run()` (`add_watermark`): Verifies correct `magick` command with gravity.
    *   [x] `run()` (failure): Simulates `subprocess.CalledProcessError`, asserts `status: error`.

5.  **`src/tools/gif_generator_tool.py`:**
    *   [x] `is_available()`: Returns `True` if `ffmpeg -version` succeeds, `False` otherwise.
    *   [ ] `run()`: Verifies correct `ffmpeg` command is formed and executed for GIF creation.
    *   [ ] `run()` (failure): Simulates `subprocess.CalledProcessError`, asserts `status: error`.

6.  **`src/tools/marketing_platform_manager.py` (and Adapters):**
    *   [ ] `deploy_campaign`: Verifies manager instantiates and calls the correct adapter method.
    *   [ ] `deploy_campaign`: Handles unknown platforms gracefully.
    *   [ ] Adapter `is_available()`: Checks relevant API keys in `os.environ`.

7.  **`src/tools/memory_retrieval.py`:**
    *   [ ] `store_memory`: Verifies `collection.add` is called with correct arguments.
    *   [ ] `retrieve_memory`: Verifies `collection.query` is called and returns mock data.

8.  **`src/tools/seo_analyzer.py`:**
    *   [ ] `calculate_keyword_density`: Correct calculation for single/multiple/case-insensitive keywords.
    *   [ ] `get_readability_scores`: Returns mocked scores.
    *   [ ] `analyze`: Combines keyword and readability results correctly.

9.  **`src/tools/competitive_analysis.py`:**
    *   [ ] `run()`: Returns expected mock competitive data for various inputs.

10. **`src/tools/code_verifier_tool.py`:**
    *   [ ] `is_available()`: Checks for CLI tool presence.
    *   [ ] `run()` (Python): Detects syntax errors, linting warnings, type errors.
    *   [ ] `run()` (Python): Detects correctly/incorrectly formatted academic citation comments.
    *   [ ] `run()` (non-Python): Basic functionality for HTML/CSS/JS.

11. **`src/tools/data_validator_tool.py`:**
    *   [ ] `is_available()`: Checks for `jsonschema` library.
    *   [ ] `run()`: Validates data against schema (success and various failure modes like missing required field, wrong type).

12. **`src/tools/data_viz_tool.py`:**
    *   [ ] `run()`: Verifies a dummy file is created at the specified path and content is plausible.

#### **IV. Agent Integration Test Plan**

1.  **All Agents (`StrategistAgent`, `ResearcherAgent`, `CopywriterAgent`, `CreativeDirectorAgent`, `CriticAgent`, `PersonaGeneratorAgent`, `ContentPlannerAgent`, `PerformanceAnalystAgent`, `CustomerAgent`):**
    *   [ ] **Initialization:** Verify `__init__` loads contexts and initializes tools.
    *   [ ] **Prompt Construction:** Verify prompt contains expected components (brand foundation, self-correction, tool descriptions, memory context, human feedback context) in correct order.
    *   [ ] **LLM Invocation:** Verify `llm.invoke` is called with expected inputs.
    *   [ ] **Output Parsing:** Verify agent correctly parses LLM output (e.g., extracts `REVISED_DRAFT`, `HTML_START`, `PERFORMANCE_REPORT_START`).
    *   [ ] **Tool Invocation (Internal):** Verify tool's `run` method is called with expected parameters.
    *   [ ] **Self-Correction Logic:** Simulate LLM errors, then corrected output. Verify agent loops and returns corrected version.
    *   [ ] **Error Handling:** Simulate unexpected LLM outputs or tool failures, verify graceful handling.

#### **V. Workflow Integration Test Plan (LangGraph Workflows)**

1.  **`CampaignWorkflow`:**
    *   [ ] **Start-to-End Flow:** Run with mock data; verify `final_state` contains expected outputs (e.g., `marketing_content`, `html_report_path`, `generated_image_path`).
    *   [ ] **Conditional Routing:** Test paths for "no image needed" vs. "image generation," "refine" vs. "end_refinement" based on mock `critic_feedback`. Test "deploy" vs. "skip deploy."
    *   [ ] **Budget Checks:** Simulate `can_generate` returning `False`, verify workflow terminates early.
    *   [ ] **Human-in-the-Loop:** Test nodes that prompt for human input, ensuring they update state correctly based on mock user input.
    *   [ ] **Error Propagation:** Simulate a critical agent/tool failure, verify workflow handles it gracefully.

2.  **`PersonaGenerationWorkflow`:**
    *   [ ] **Start-to-End Flow:** Run with mock data; verify `generated_persona` and `persona_name` in `final_state`.
    *   [ ] **Tool Usage:** Verify `MarketResearchTool` is invoked if available.
    *   [ ] **Storage:** Verify `MemoryRetrievalTool.run` is called to store the persona.

3.  **`ContentCalendarWorkflow`:**
    *   [ ] **Start-to-End Flow:** Run with mock data; verify `generated_calendar_md` in `final_state`.
    *   [ ] **Tool Usage:** Verify `MemoryRetrievalTool` and `EventCalendarTool` are invoked.
    *   [ ] **Email Schedule:** Verify `ScheduledTaskRegistry.add_schedule` is called if email updates are enabled.

4.  **`CampaignPerformanceWorkflow`:**
    *   [ ] **Start-to-End Flow:** Run with mock data; verify `final_report_md` and `generated_charts_paths` in `final_state`.
    *   [ ] **Tool Usage:** Verify `MetricsFetcherTool`, `DataVizTool`, `DataValidatorTool` are invoked.
    *   [ ] **Code Verification:** Verify `CodeVerifierTool` is conceptually used on generated analysis code.
    *   [ ] **Storage:** Verify `MemoryRetrievalTool.run` is called to store the report.

5.  **`WidgetInnovationWorkflow`:**
    *   [ ] **Start-to-End Flow:** Run with mock data; verify `widget_proposal_md`, `widget_code_html`/`css`/`js`, `widget_marketing_plan_md` in `final_state`.
    *   [ ] **Tool Usage:** Verify `CodeVerifierTool` is invoked for self-correction.
    *   [ ] **Code Verification Loop:** Simulate initial code errors, verify agent attempts self-correction.

6.  **`CustomerFocusGroupWorkflow`:**
    *   [ ] **Start-to-End Flow:** Run with mock data; verify `feedback_results` (from multiple agents) and `summary_feedback` in `final_state`.
    *   [ ] **Agent Instantiation:** Verify multiple `CustomerAgent` instances are created with different persona configs.

#### **VI. End-to-End "Black Box" Test Plan (Simulated)**

1.  **`run_campaign.py` (Main CLI):**
    *   [ ] Verify CLI arguments are correctly parsed and passed to `run_campaign` (e.g., `objective`, `word_length`, `deploy-platform`, `enable-feedback`).
    *   [ ] Verify `set_current_campaign_id` is called.
    *   [ ] Verify final summary output to console is correct based on mocked `final_state`.

2.  **`run_daily_calendar_digest.py` (Scheduled Script):**
    *   [ ] Verify it iterates through active schedules (from mocked `ScheduledTaskRegistry`).
    *   [ ] Verify it correctly fetches and parses the calendar (from mocked `MemoryRetrievalTool`).
    *   [ ] Verify it filters tasks for the correct daily/weekly period.
    *   [ ] Verify `EmailSenderTool.run` is called with the correct email content.