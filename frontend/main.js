document.addEventListener('DOMContentLoaded', () => {
    const toolsSidebar = document.getElementById('tools-sidebar');
    const selectedToolTitle = document.getElementById('selected-tool-title');
    const toolInteractionArea = document.getElementById('tool-interaction-area');
    const outputDisplay = document.getElementById('output-display');

    let selectedTool = null;
    let availableTools = [];
    let eventUnsubs = [];

    setupEventListeners();
    initialize();

    async function initialize() {
        await loadTools();
        renderTools();
        if (availableTools.length > 0) {
            selectTool(availableTools[0]);
        }
    }

    function setOutput(text) {
        outputDisplay.textContent = text;
    }

    function appendOutput(text) {
        outputDisplay.textContent += `\n${text}`;
    }

    async function setupEventListeners() {
        const listen = window.__TAURI__?.event?.listen;
        if (!listen) {
            console.warn('Tauri event API not found; live job updates disabled.');
            return;
        }

        eventUnsubs.push(await listen('tool-job-progress', (event) => {
            const payload = event.payload;
            appendOutput(`[progress] ${payload.job_id} ${payload.progress_pct}% ${payload.stage}`);
        }));

        eventUnsubs.push(await listen('tool-job-completed', (event) => {
            const payload = event.payload;
            appendOutput(`[completed] ${payload.job_id}`);
        }));

        eventUnsubs.push(await listen('tool-job-failed', (event) => {
            const payload = event.payload;
            appendOutput(`[failed] ${payload.job_id} ${payload.message || 'execution failed'}`);
        }));
    }

    async function loadTools() {
        const localSpecialTools = [
            {
                name: 'mvp_pipeline_runner',
                description: 'Runs the MVP cross-tool pipeline (competitive analysis -> SEO analyzer -> data visualization).',
                ui_metadata: {
                    category: 'Pipelines',
                    display_name: 'MVP Pipeline Runner',
                    tags: ['pipeline', 'mvp', 'orchestration'],
                    estimated_time_seconds: 90
                },
                parameters: [
                    { name: 'topic', type: 'string', optional: false, description: 'Market analysis topic query.' },
                    { name: 'max_sources', type: 'number', optional: true, description: 'Maximum sources to analyze (3-20).' },
                    { name: 'chart_output_path', type: 'string', optional: false, description: 'Absolute path for generated chart output.' }
                ],
                input_examples: [
                    {
                        topic: 'freeze dried raw dog food for sensitive stomachs',
                        max_sources: 8,
                        chart_output_path: '/tmp/mvp_market_keyword_chart.png'
                    }
                ]
            },
            {
                name: 'generate_image',
                description: 'Generates an image using Google Gemini.',
                ui_metadata: {
                    category: 'Creative',
                    display_name: 'Generate Image',
                    tags: ['image', 'creative'],
                    estimated_time_seconds: 15
                },
                parameters: [
                    { name: 'prompt', type: 'string', optional: false, description: 'Prompt for image generation' },
                    { name: 'campaign_dir', type: 'string', optional: false, description: 'Directory for image output' }
                ],
                input_examples: [
                    {
                        prompt: 'A healthy energetic dog enjoying a raw meal in warm natural light',
                        campaign_dir: '/tmp/campaign'
                    }
                ]
            }
        ];

        try {
            const backendTools = await window.__TAURI__.core.invoke('get_tools');
            const normalized = backendTools.map((tool) => ({
                ...tool,
                parameters: (tool.parameters || []).map((p) => ({
                    ...p,
                    type: p.type || p["type"] || 'string'
                }))
            }));

            availableTools = [...localSpecialTools, ...normalized];
            sortToolsByRecentAndName();
        } catch (error) {
            setOutput(`Failed to load tools dynamically: ${error}`);
            availableTools = localSpecialTools;
        }
    }

    function getRecentTools() {
        try {
            return JSON.parse(localStorage.getItem('recentTools') || '[]');
        } catch (_) {
            return [];
        }
    }

    function markToolAsRecent(toolName) {
        const recent = getRecentTools().filter((name) => name !== toolName);
        recent.unshift(toolName);
        localStorage.setItem('recentTools', JSON.stringify(recent.slice(0, 5)));
    }

    function sortToolsByRecentAndName() {
        const recent = getRecentTools();
        const recentSet = new Set(recent);
        availableTools.sort((a, b) => {
            const aRecent = recentSet.has(a.name);
            const bRecent = recentSet.has(b.name);
            if (aRecent && !bRecent) return -1;
            if (!aRecent && bRecent) return 1;
            return displayName(a).localeCompare(displayName(b));
        });
    }

    function displayName(tool) {
        return tool.ui_metadata?.display_name || tool.name;
    }

    function renderTools() {
        toolsSidebar.innerHTML = '';

        const grouped = {};
        for (const tool of availableTools) {
            const category = tool.ui_metadata?.category || 'Uncategorized';
            if (!grouped[category]) {
                grouped[category] = [];
            }
            grouped[category].push(tool);
        }

        const categories = Object.keys(grouped).sort();
        for (const category of categories) {
            const categoryItem = document.createElement('li');
            categoryItem.textContent = category;
            categoryItem.className = 'category-label';
            toolsSidebar.appendChild(categoryItem);

            for (const tool of grouped[category]) {
                const listItem = document.createElement('li');
                listItem.textContent = `- ${displayName(tool)}`;
                listItem.dataset.toolName = tool.name;
                listItem.addEventListener('click', () => selectTool(tool));
                toolsSidebar.appendChild(listItem);
            }
        }
    }

    function selectTool(tool) {
        selectedTool = tool;
        selectedToolTitle.textContent = `Tool: ${displayName(tool)}`;
        setOutput(tool.description || '');

        Array.from(toolsSidebar.querySelectorAll('li')).forEach((item) => {
            if (item.dataset.toolName === tool.name) {
                item.classList.add('selected');
            } else {
                item.classList.remove('selected');
            }
        });

        renderToolInteractionArea(tool);
    }

    function renderToolInteractionArea(tool) {
        toolInteractionArea.innerHTML = '';

        const form = document.createElement('form');
        let selectedCampaignDirPath = '';
        const inputExample = (tool.input_examples && tool.input_examples[0]) || {};

        form.addEventListener('submit', async (e) => {
            e.preventDefault();
            const params = {};

            for (const param of (tool.parameters || [])) {
                if (param.name === 'campaign_dir') {
                    if (selectedCampaignDirPath) {
                        params.campaign_dir = selectedCampaignDirPath;
                    } else {
                        alert('Please select a campaign directory.');
                        return;
                    }
                    continue;
                }

                const input = form.elements[param.name];
                if (!input) continue;

                const rawValue = input.value;
                if (!rawValue && !param.optional) {
                    alert(`Please provide ${param.name}.`);
                    return;
                }

                if (!rawValue) continue;

                if (param.type === 'number' || param.type === 'integer') {
                    params[param.name] = Number(rawValue);
                    continue;
                }

                if (param.type === 'array') {
                    params[param.name] = rawValue
                        .split(',')
                        .map((s) => s.trim())
                        .filter(Boolean);
                    continue;
                }

                if (param.type === 'json') {
                    try {
                        params[param.name] = JSON.parse(rawValue);
                    } catch (err) {
                        alert(`Invalid JSON in ${param.name}: ${err}`);
                        return;
                    }
                    continue;
                }

                params[param.name] = rawValue;
            }

            markToolAsRecent(tool.name);
            sortToolsByRecentAndName();
            renderTools();
            await executeTool(tool.name, params);
        });

        for (const param of (tool.parameters || [])) {
            const label = document.createElement('label');
            const required = !param.optional;
            label.textContent = `${param.name}${required ? '*' : ''}:`;
            form.appendChild(label);

            if (param.name === 'campaign_dir') {
                const chooseDirButton = document.createElement('button');
                chooseDirButton.type = 'button';
                chooseDirButton.textContent = 'Choose Campaign Directory';
                chooseDirButton.addEventListener('click', async () => {
                    try {
                        const selected = await window.__TAURI__.dialog.open({
                            directory: true,
                            multiple: false,
                            defaultPath: (window.__TAURI__?.path?.documentDir && await window.__TAURI__.path.documentDir()) || '/'
                        });
                        if (selected) {
                            selectedCampaignDirPath = selected;
                            const selectedPathSpan = form.querySelector('#selected-campaign-dir-display');
                            selectedPathSpan.textContent = selected;
                        }
                    } catch (err) {
                        alert(`Error choosing directory: ${err}`);
                    }
                });
                form.appendChild(chooseDirButton);

                const selectedPathSpan = document.createElement('span');
                selectedPathSpan.id = 'selected-campaign-dir-display';
                selectedPathSpan.style.marginLeft = '10px';
                selectedPathSpan.textContent = 'No directory selected';
                form.appendChild(selectedPathSpan);
                continue;
            }

            const inputElement = param.type === 'json' ? document.createElement('textarea') : document.createElement('input');
            if (param.type !== 'json') {
                inputElement.type = (param.type === 'number' || param.type === 'integer') ? 'number' : 'text';
            }
            inputElement.name = param.name;
            inputElement.placeholder = param.description || `Enter ${param.name}`;
            inputElement.required = required;

            if (inputExample[param.name] !== undefined) {
                inputElement.value = Array.isArray(inputExample[param.name])
                    ? inputExample[param.name].join(', ')
                    : (typeof inputExample[param.name] === 'object'
                        ? JSON.stringify(inputExample[param.name], null, 2)
                        : String(inputExample[param.name]));
            }

            form.appendChild(inputElement);
        }

        const executeButton = document.createElement('button');
        executeButton.type = 'submit';
        executeButton.textContent = 'Execute Tool';
        form.appendChild(executeButton);

        toolInteractionArea.appendChild(form);
    }

    async function executeTool(toolName, params) {
        setOutput(`Executing ${toolName} with parameters:\n${JSON.stringify(params, null, 2)}\n`);

        try {
            if (toolName === 'mvp_pipeline_runner') {
                const definition = buildMvpPipelineDefinition(params);
                const handle = await window.__TAURI__.core.invoke('start_pipeline_job', { definition });
                appendOutput(`Started pipeline job ${handle.job_id}`);

                while (true) {
                    const snapshot = await window.__TAURI__.core.invoke('get_tool_job', { jobId: handle.job_id });

                    if (snapshot.status === 'succeeded') {
                        appendOutput('Pipeline completed successfully.');
                        appendPipelineSummary(snapshot.output || {});
                        appendOutput(`Raw pipeline output:\n${JSON.stringify(snapshot.output, null, 2)}`);
                        break;
                    }

                    if (snapshot.status === 'failed' || snapshot.status === 'canceled') {
                        appendOutput(`Pipeline ended with status: ${snapshot.status}`);
                        appendOutput(`Error:\n${JSON.stringify(snapshot.error, null, 2)}`);
                        break;
                    }

                    await new Promise((resolve) => setTimeout(resolve, 300));
                }
                return;
            }

            if (toolName === 'generate_image') {
                const result = await window.__TAURI__.core.invoke('generate_image_command', {
                    prompt: params.prompt,
                    campaignDir: params.campaign_dir
                });
                appendOutput(`Generated image: ${result}`);
                return;
            }

            const handle = await window.__TAURI__.core.invoke('start_tool_job', {
                toolName,
                input: params
            });

            appendOutput(`Started job ${handle.job_id}`);

            while (true) {
                const snapshot = await window.__TAURI__.core.invoke('get_tool_job', { jobId: handle.job_id });

                if (snapshot.status === 'succeeded') {
                    appendOutput(`Result:\n${JSON.stringify(snapshot.output, null, 2)}`);
                    break;
                }

                if (snapshot.status === 'failed' || snapshot.status === 'canceled') {
                    appendOutput(`Error:\n${JSON.stringify(snapshot.error, null, 2)}`);
                    break;
                }

                await new Promise((resolve) => setTimeout(resolve, 300));
            }
        } catch (error) {
            appendOutput(`Execution error: ${error}`);
        }
    }

    function buildMvpPipelineDefinition(params) {
        const maxSourcesRaw = Number(params.max_sources);
        const maxSources = Number.isFinite(maxSourcesRaw) ? Math.max(3, Math.min(20, maxSourcesRaw)) : 8;
        const topic = params.topic;
        const chartOutputPath = params.chart_output_path;

        return {
            name: 'market_analysis_mvp_v1',
            campaign_id: 'frontend_mvp_pipeline',
            steps: [
                {
                    id: 'market_signals',
                    tool: 'competitive_analysis',
                    input: {
                        topic,
                        max_sources: maxSources
                    }
                },
                {
                    id: 'seo_quality',
                    tool: 'seo_analyzer',
                    input: {
                        text: {
                            from_step: 'market_signals',
                            path: '/signal_report_markdown'
                        },
                        keywords: ['raw dog food', 'sensitive stomach', 'nutrition']
                    }
                },
                {
                    id: 'signal_chart',
                    tool: 'data_viz',
                    input: {
                        data: {
                            from_step: 'market_signals',
                            path: '/keyword_frequency'
                        },
                        chart_type: 'bar',
                        output_path: chartOutputPath
                    }
                }
            ]
        };
    }

    function appendPipelineSummary(result) {
        appendOutput(`Pipeline: ${result.pipeline_name}`);
        appendOutput(`Succeeded: ${result.succeeded}`);
        appendOutput(`Started: ${result.started_at}`);
        appendOutput(`Finished: ${result.finished_at}`);
        appendOutput('Steps:');
        for (const step of (result.steps || [])) {
            const errorMessage = step.error?.message ? ` | error: ${step.error.message}` : '';
            appendOutput(`- ${step.step_id} [${step.status}] (${step.duration_ms}ms)${errorMessage}`);
        }
    }

    window.addEventListener('beforeunload', () => {
        eventUnsubs.forEach((unsub) => {
            if (typeof unsub === 'function') {
                unsub();
            }
        });
    });
});
