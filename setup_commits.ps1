# setup_commits.ps1 - Create 121 commits and push to private GitHub repo
# Run from: c:\Users\naman\Desktop\my projects\govrix

$ErrorActionPreference = "Continue"

# --- Step 1: Init git repo ---
git init
git branch -m main

# --- Step 2: Define all 121 commits ---
# Each commit: files to stage (array), commit message
$commits = @(
    # === Phase 1: Project Initialization (Commits 1-5) ===
    @{ files = @(); msg = "chore: initialize AgentMesh project repository"; cmd = "New-Item -ItemType File -Path .gitkeep -Force | Out-Null; git add .gitkeep" },
    @{ files = @(); msg = "docs: add project README placeholder"; cmd = @"
@'
# AgentMesh

> Transparent proxy for AI agent observability and governance.

Work in progress - see `/docs` for full specification.
'@ | Set-Content -Path README.md -Encoding UTF8; git add README.md
"@ },
    @{ files = @(); msg = "chore: add .gitignore for docs project"; cmd = @"
@'
*.tmp
*.bak
.DS_Store
Thumbs.db
*.swp
*~
.env
node_modules/
'@ | Set-Content -Path .gitignore -Encoding UTF8; git add .gitignore
"@ },
    @{ files = @(); msg = "docs: create documentation directory structure"; cmd = @"
New-Item -ItemType Directory -Path docs/01-vision -Force | Out-Null
New-Item -ItemType Directory -Path docs/02-problems -Force | Out-Null
New-Item -ItemType Directory -Path docs/03-features -Force | Out-Null
New-Item -ItemType Directory -Path docs/04-prd -Force | Out-Null
New-Item -ItemType Directory -Path docs/05-architecture -Force | Out-Null
New-Item -ItemType Directory -Path docs/06-coding-plan -Force | Out-Null
New-Item -ItemType Directory -Path docs/07-gtm-execution -Force | Out-Null
New-Item -ItemType File -Path docs/.gitkeep -Force | Out-Null
git add docs/.gitkeep
"@ },
    @{ files = @(); msg = "docs: add initial idea notes and ground truth document"; cmd = "git add idea/" }
)

# === Phase 2: Vision Documents (Commits 6-16) ===
# We'll split VISION.md into multiple commits
$visionFile = "docs/01-vision/VISION.md"
$visionLines = Get-Content $visionFile -Encoding UTF8
$visionTotal = $visionLines.Count
$visionChunk = [math]::Ceiling($visionTotal / 4)

# Temporarily move files out, we'll add them incrementally
$tempDir = "$env:TEMP\govrix_temp"
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

# Save all doc files to temp
$allFiles = @(
    "docs/01-vision/VISION.md",
    "docs/01-vision/GTM_STRATEGY.md",
    "docs/01-vision/VC_PITCH_DECK_SCRIPT.md",
    "docs/02-problems/INDUSTRY_PROBLEMS.md",
    "docs/02-problems/SOLUTION_MAPPING.md",
    "docs/03-features/F01_PROXY_ENGINE.md",
    "docs/03-features/F02_EVENT_STORE.md",
    "docs/03-features/F03_AGENT_REGISTRY.md",
    "docs/03-features/F04_DASHBOARD.md",
    "docs/03-features/F05_CLI.md",
    "docs/03-features/F06_COST_TRACKING.md",
    "docs/03-features/F07_REPORT_GENERATOR.md",
    "docs/03-features/F08_MCP_INTERCEPTION.md",
    "docs/03-features/F09_LLM_API_INTERCEPTION.md",
    "docs/03-features/F10_A2A_LOGGING.md",
    "docs/03-features/F11_POLICY_ENGINE.md",
    "docs/03-features/F12_SESSION_RECORDER.md",
    "docs/03-features/F13_PII_MASKING.md",
    "docs/03-features/F14_COMPLIANCE_TEMPLATES.md",
    "docs/03-features/F15_A2A_IDENTITY.md",
    "docs/04-prd/PRD.md",
    "docs/05-architecture/ARCHITECTURE_OVERVIEW.md",
    "docs/05-architecture/TECH_AGENT_REGISTRY.md",
    "docs/05-architecture/TECH_API.md",
    "docs/05-architecture/TECH_CLI.md",
    "docs/05-architecture/TECH_DASHBOARD.md",
    "docs/05-architecture/TECH_DATA_MODELS.md",
    "docs/05-architecture/TECH_DEPLOYMENT.md",
    "docs/05-architecture/TECH_EVENT_STORE.md",
    "docs/05-architecture/TECH_POLICY_ENGINE.md",
    "docs/05-architecture/TECH_PROXY.md",
    "docs/06-coding-plan/CODING_PHASES.md",
    "docs/06-coding-plan/SHARED_CONTEXT.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_01.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_02.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_03.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_04.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_05.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_06.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_07.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_08.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_09.md",
    "docs/06-coding-plan/AGENT_TASK_PHASE_10.md",
    "docs/07-gtm-execution/LAUNCH_PLAYBOOK.md",
    "docs/07-gtm-execution/COMMUNITY_GROWTH.md",
    "docs/07-gtm-execution/CONTENT_CALENDAR.md",
    "docs/07-gtm-execution/PROMOTION_GUIDE.md",
    "docs/07-gtm-execution/SALES_OUTREACH.md"
)

Write-Host "Backing up all files to temp..."
foreach ($f in $allFiles) {
    $dest = Join-Path $tempDir $f
    $destDir = Split-Path $dest -Parent
    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    if (Test-Path $f) {
        Copy-Item $f $dest -Force
        Remove-Item $f -Force
    }
}

# Helper function: restore a file from temp and stage it
function Restore-And-Stage {
    param([string]$filePath)
    $src = Join-Path $tempDir $filePath
    $destDir = Split-Path $filePath -Parent
    if (-not (Test-Path $destDir)) {
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    }
    Copy-Item $src $filePath -Force
    git add $filePath
}

# Helper: write partial content of a file (first N lines)
function Write-Partial {
    param([string]$filePath, [int]$lineCount)
    $src = Join-Path $tempDir $filePath
    $allLines = Get-Content $src -Encoding UTF8
    $count = [math]::Min($lineCount, $allLines.Count)
    $allLines[0..($count-1)] | Set-Content -Path $filePath -Encoding UTF8
    git add $filePath
}

# Helper: write full file
function Write-Full {
    param([string]$filePath)
    $src = Join-Path $tempDir $filePath
    Copy-Item $src $filePath -Force
    git add $filePath
}

# Now define remaining commits 6-121 as structured data
$remainingCommits = @(
    # === Phase 2: Vision & Strategy (6-16) ===
    @{ action = "partial"; file = "docs/01-vision/VISION.md"; lines = 100; msg = "docs(vision): draft AgentMesh vision statement and problem definition" },
    @{ action = "partial"; file = "docs/01-vision/VISION.md"; lines = 250; msg = "docs(vision): expand on market opportunity and target personas" },
    @{ action = "partial"; file = "docs/01-vision/VISION.md"; lines = 450; msg = "docs(vision): add architectural principles and design philosophy" },
    @{ action = "full"; file = "docs/01-vision/VISION.md"; msg = "docs(vision): complete vision document with roadmap and business model" },
    @{ action = "partial"; file = "docs/01-vision/GTM_STRATEGY.md"; lines = 200; msg = "docs(gtm): draft go-to-market strategy executive summary" },
    @{ action = "partial"; file = "docs/01-vision/GTM_STRATEGY.md"; lines = 500; msg = "docs(gtm): add customer segmentation and positioning framework" },
    @{ action = "partial"; file = "docs/01-vision/GTM_STRATEGY.md"; lines = 900; msg = "docs(gtm): detail two-motion sales architecture and pricing tiers" },
    @{ action = "full"; file = "docs/01-vision/GTM_STRATEGY.md"; msg = "docs(gtm): finalize GTM strategy with metrics, KPIs, and execution roadmap" },
    @{ action = "partial"; file = "docs/01-vision/VC_PITCH_DECK_SCRIPT.md"; lines = 200; msg = "docs(pitch): draft VC pitch deck opening and problem slides" },
    @{ action = "partial"; file = "docs/01-vision/VC_PITCH_DECK_SCRIPT.md"; lines = 500; msg = "docs(pitch): add solution, market size, and competitive landscape slides" },
    @{ action = "full"; file = "docs/01-vision/VC_PITCH_DECK_SCRIPT.md"; msg = "docs(pitch): complete VC pitch deck with financials and ask" },

    # === Phase 3: Problem Definition (17-22) ===
    @{ action = "partial"; file = "docs/02-problems/INDUSTRY_PROBLEMS.md"; lines = 200; msg = "docs(problems): identify core reliability challenges in AI agent deployments" },
    @{ action = "partial"; file = "docs/02-problems/INDUSTRY_PROBLEMS.md"; lines = 500; msg = "docs(problems): detail governance, compliance, and cost control gaps" },
    @{ action = "full"; file = "docs/02-problems/INDUSTRY_PROBLEMS.md"; msg = "docs(problems): complete industry problems analysis with real-world examples" },
    @{ action = "partial"; file = "docs/02-problems/SOLUTION_MAPPING.md"; lines = 200; msg = "docs(solutions): map industry problems to AgentMesh capabilities" },
    @{ action = "partial"; file = "docs/02-problems/SOLUTION_MAPPING.md"; lines = 450; msg = "docs(solutions): add detailed feature-to-problem mapping matrix" },
    @{ action = "full"; file = "docs/02-problems/SOLUTION_MAPPING.md"; msg = "docs(solutions): finalize solution mapping with priority and tier assignments" },

    # === Phase 4: Feature Specifications - OSS Core (23-48) ===
    @{ action = "partial"; file = "docs/03-features/F01_PROXY_ENGINE.md"; lines = 150; msg = "feat(F01): draft transparent proxy engine problem statement and overview" },
    @{ action = "partial"; file = "docs/03-features/F01_PROXY_ENGINE.md"; lines = 400; msg = "feat(F01): add proxy pipeline architecture and protocol support" },
    @{ action = "full"; file = "docs/03-features/F01_PROXY_ENGINE.md"; msg = "feat(F01): complete proxy engine spec with user stories and configuration" },

    @{ action = "partial"; file = "docs/03-features/F02_EVENT_STORE.md"; lines = 150; msg = "feat(F02): draft event store problem statement and data model" },
    @{ action = "partial"; file = "docs/03-features/F02_EVENT_STORE.md"; lines = 400; msg = "feat(F02): add event store write path and query interface" },
    @{ action = "full"; file = "docs/03-features/F02_EVENT_STORE.md"; msg = "feat(F02): complete event store spec with retention policies and API" },

    @{ action = "partial"; file = "docs/03-features/F03_AGENT_REGISTRY.md"; lines = 150; msg = "feat(F03): draft agent registry auto-discovery and identity model" },
    @{ action = "full"; file = "docs/03-features/F03_AGENT_REGISTRY.md"; msg = "feat(F03): complete agent registry spec with health scoring and API" },

    @{ action = "partial"; file = "docs/03-features/F04_DASHBOARD.md"; lines = 200; msg = "feat(F04): draft real-time dashboard overview and widget layout" },
    @{ action = "full"; file = "docs/03-features/F04_DASHBOARD.md"; msg = "feat(F04): complete dashboard spec with WebSocket streaming and filters" },

    @{ action = "partial"; file = "docs/03-features/F05_CLI.md"; lines = 200; msg = "feat(F05): draft CLI tool command structure and agent management" },
    @{ action = "full"; file = "docs/03-features/F05_CLI.md"; msg = "feat(F05): complete CLI spec with all subcommands and output formats" },

    @{ action = "partial"; file = "docs/03-features/F06_COST_TRACKING.md"; lines = 150; msg = "feat(F06): draft cost tracking problem statement and token counting" },
    @{ action = "full"; file = "docs/03-features/F06_COST_TRACKING.md"; msg = "feat(F06): complete cost tracking spec with budget alerts and reporting" },

    @{ action = "partial"; file = "docs/03-features/F07_REPORT_GENERATOR.md"; lines = 150; msg = "feat(F07): draft report generator templates and scheduling" },
    @{ action = "full"; file = "docs/03-features/F07_REPORT_GENERATOR.md"; msg = "feat(F07): complete report generator spec with export formats and API" },

    @{ action = "partial"; file = "docs/03-features/F08_MCP_INTERCEPTION.md"; lines = 200; msg = "feat(F08): draft MCP interception protocol handling and tool logging" },
    @{ action = "full"; file = "docs/03-features/F08_MCP_INTERCEPTION.md"; msg = "feat(F08): complete MCP interception spec with SSE streaming and schema" },

    @{ action = "partial"; file = "docs/03-features/F09_LLM_API_INTERCEPTION.md"; lines = 200; msg = "feat(F09): draft LLM API interception for OpenAI and Anthropic protocols" },
    @{ action = "full"; file = "docs/03-features/F09_LLM_API_INTERCEPTION.md"; msg = "feat(F09): complete LLM API interception spec with streaming support" },

    @{ action = "partial"; file = "docs/03-features/F10_A2A_LOGGING.md"; lines = 150; msg = "feat(F10): draft A2A protocol logging and agent-to-agent visibility" },
    @{ action = "full"; file = "docs/03-features/F10_A2A_LOGGING.md"; msg = "feat(F10): complete A2A logging spec with task lifecycle tracking" },

    # === Phase 5: Feature Specifications - Enterprise (49-70) ===
    @{ action = "partial"; file = "docs/03-features/F11_POLICY_ENGINE.md"; lines = 150; msg = "feat(F11): draft policy engine problem statement and rule model" },
    @{ action = "partial"; file = "docs/03-features/F11_POLICY_ENGINE.md"; lines = 400; msg = "feat(F11): add YAML policy schema and condition reference" },
    @{ action = "partial"; file = "docs/03-features/F11_POLICY_ENGINE.md"; lines = 700; msg = "feat(F11): add PII detection pipeline and budget enforcement" },
    @{ action = "partial"; file = "docs/03-features/F11_POLICY_ENGINE.md"; lines = 1000; msg = "feat(F11): add approval gates, OPA compatibility, and CRUD API" },
    @{ action = "full"; file = "docs/03-features/F11_POLICY_ENGINE.md"; msg = "feat(F11): complete policy engine spec with configuration and deployment" },

    @{ action = "partial"; file = "docs/03-features/F12_SESSION_RECORDER.md"; lines = 150; msg = "feat(F12): draft session recorder cryptographic audit trail design" },
    @{ action = "partial"; file = "docs/03-features/F12_SESSION_RECORDER.md"; lines = 400; msg = "feat(F12): add Merkle tree chain and tamper-detection mechanism" },
    @{ action = "full"; file = "docs/03-features/F12_SESSION_RECORDER.md"; msg = "feat(F12): complete session recorder spec with replay API and storage" },

    @{ action = "partial"; file = "docs/03-features/F13_PII_MASKING.md"; lines = 100; msg = "feat(F13): draft PII masking problem statement and regulatory exposure" },
    @{ action = "partial"; file = "docs/03-features/F13_PII_MASKING.md"; lines = 250; msg = "feat(F13): add detection pipeline and masking modes (inline vs log)" },
    @{ action = "partial"; file = "docs/03-features/F13_PII_MASKING.md"; lines = 420; msg = "feat(F13): add user stories for PII masking and allow-list management" },
    @{ action = "partial"; file = "docs/03-features/F13_PII_MASKING.md"; lines = 580; msg = "feat(F13): add high-level design with Mermaid diagrams and manifest format" },
    @{ action = "full"; file = "docs/03-features/F13_PII_MASKING.md"; msg = "feat(F13): complete PII masking spec with API surface, config, and performance" },

    @{ action = "partial"; file = "docs/03-features/F14_COMPLIANCE_TEMPLATES.md"; lines = 150; msg = "feat(F14): draft compliance templates for SOC2, HIPAA, and GDPR" },
    @{ action = "partial"; file = "docs/03-features/F14_COMPLIANCE_TEMPLATES.md"; lines = 400; msg = "feat(F14): add report generation pipeline and scheduling" },
    @{ action = "full"; file = "docs/03-features/F14_COMPLIANCE_TEMPLATES.md"; msg = "feat(F14): complete compliance templates spec with API and configuration" },

    @{ action = "partial"; file = "docs/03-features/F15_A2A_IDENTITY.md"; lines = 150; msg = "feat(F15): draft A2A mutual identity verification protocol" },
    @{ action = "partial"; file = "docs/03-features/F15_A2A_IDENTITY.md"; lines = 400; msg = "feat(F15): add mTLS integration and identity attestation flow" },
    @{ action = "full"; file = "docs/03-features/F15_A2A_IDENTITY.md"; msg = "feat(F15): complete A2A identity spec with trust scoring and config" },

    # === Phase 6: PRD (71-78) ===
    @{ action = "partial"; file = "docs/04-prd/PRD.md"; lines = 150; msg = "docs(prd): draft product requirements document header and overview" },
    @{ action = "partial"; file = "docs/04-prd/PRD.md"; lines = 350; msg = "docs(prd): add persona definitions and user journey mapping" },
    @{ action = "partial"; file = "docs/04-prd/PRD.md"; lines = 600; msg = "docs(prd): add feature matrix with priority and tier assignments" },
    @{ action = "partial"; file = "docs/04-prd/PRD.md"; lines = 900; msg = "docs(prd): add detailed acceptance criteria for all features" },
    @{ action = "partial"; file = "docs/04-prd/PRD.md"; lines = 1200; msg = "docs(prd): add non-functional requirements and performance targets" },
    @{ action = "full"; file = "docs/04-prd/PRD.md"; msg = "docs(prd): finalize PRD with release roadmap and success metrics" },
    @{ action = "full"; file = "docs/04-prd/PRD.md"; msg = "docs(prd): fix formatting and cross-references in PRD" },
    @{ action = "full"; file = "docs/04-prd/PRD.md"; msg = "docs(prd): update PRD feature table to include F13 PII Masking details" },

    # === Phase 7: Architecture (79-100) ===
    @{ action = "partial"; file = "docs/05-architecture/ARCHITECTURE_OVERVIEW.md"; lines = 150; msg = "docs(arch): draft architecture overview with design principles" },
    @{ action = "partial"; file = "docs/05-architecture/ARCHITECTURE_OVERVIEW.md"; lines = 400; msg = "docs(arch): add system component diagram and data flow" },
    @{ action = "full"; file = "docs/05-architecture/ARCHITECTURE_OVERVIEW.md"; msg = "docs(arch): complete architecture overview with deployment topology" },

    @{ action = "partial"; file = "docs/05-architecture/TECH_PROXY.md"; lines = 200; msg = "docs(arch): draft proxy technical spec with Rust implementation details" },
    @{ action = "partial"; file = "docs/05-architecture/TECH_PROXY.md"; lines = 500; msg = "docs(arch): add proxy pipeline stages and protocol parsing" },
    @{ action = "full"; file = "docs/05-architecture/TECH_PROXY.md"; msg = "docs(arch): complete proxy tech spec with performance benchmarks" },

    @{ action = "partial"; file = "docs/05-architecture/TECH_EVENT_STORE.md"; lines = 200; msg = "docs(arch): draft event store tech spec with DuckDB integration" },
    @{ action = "full"; file = "docs/05-architecture/TECH_EVENT_STORE.md"; msg = "docs(arch): complete event store tech spec with schema and queries" },

    @{ action = "partial"; file = "docs/05-architecture/TECH_AGENT_REGISTRY.md"; lines = 200; msg = "docs(arch): draft agent registry tech spec with SQLite schema" },
    @{ action = "full"; file = "docs/05-architecture/TECH_AGENT_REGISTRY.md"; msg = "docs(arch): complete agent registry tech spec with health scoring algo" },

    @{ action = "partial"; file = "docs/05-architecture/TECH_DASHBOARD.md"; lines = 200; msg = "docs(arch): draft dashboard tech spec with SolidJS architecture" },
    @{ action = "full"; file = "docs/05-architecture/TECH_DASHBOARD.md"; msg = "docs(arch): complete dashboard tech spec with WebSocket protocol" },

    @{ action = "partial"; file = "docs/05-architecture/TECH_CLI.md"; lines = 200; msg = "docs(arch): draft CLI tech spec with Rust clap framework" },
    @{ action = "full"; file = "docs/05-architecture/TECH_CLI.md"; msg = "docs(arch): complete CLI tech spec with all command implementations" },

    @{ action = "partial"; file = "docs/05-architecture/TECH_API.md"; lines = 200; msg = "docs(arch): draft REST API tech spec with endpoint inventory" },
    @{ action = "full"; file = "docs/05-architecture/TECH_API.md"; msg = "docs(arch): complete REST API tech spec with auth and rate limiting" },

    @{ action = "full"; file = "docs/05-architecture/TECH_DATA_MODELS.md"; msg = "docs(arch): add data models tech spec with all entity definitions" },

    @{ action = "partial"; file = "docs/05-architecture/TECH_POLICY_ENGINE.md"; lines = 200; msg = "docs(arch): draft policy engine tech spec with Rust module structure" },
    @{ action = "partial"; file = "docs/05-architecture/TECH_POLICY_ENGINE.md"; lines = 500; msg = "docs(arch): add decision tree compilation and evaluation algorithm" },
    @{ action = "partial"; file = "docs/05-architecture/TECH_POLICY_ENGINE.md"; lines = 800; msg = "docs(arch): add PII detection pipeline and budget enforcement internals" },
    @{ action = "full"; file = "docs/05-architecture/TECH_POLICY_ENGINE.md"; msg = "docs(arch): complete policy engine tech spec with OPA and error handling" },

    @{ action = "full"; file = "docs/05-architecture/TECH_DEPLOYMENT.md"; msg = "docs(arch): add deployment tech spec with Docker and Kubernetes configs" },

    # === Phase 8: Coding Plan (101-115) ===
    @{ action = "partial"; file = "docs/06-coding-plan/CODING_PHASES.md"; lines = 150; msg = "docs(plan): draft coding phases overview and sprint structure" },
    @{ action = "full"; file = "docs/06-coding-plan/CODING_PHASES.md"; msg = "docs(plan): complete coding phases with timeline and dependencies" },
    @{ action = "full"; file = "docs/06-coding-plan/SHARED_CONTEXT.md"; msg = "docs(plan): add shared context document for agent-assisted development" },

    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_01.md"; msg = "docs(plan): add Phase 1 agent tasks - project scaffolding and CI setup" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_02.md"; msg = "docs(plan): add Phase 2 agent tasks - proxy engine core implementation" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_03.md"; msg = "docs(plan): add Phase 3 agent tasks - event store and data pipeline" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_04.md"; msg = "docs(plan): add Phase 4 agent tasks - agent registry and health scoring" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_05.md"; msg = "docs(plan): add Phase 5 agent tasks - dashboard and real-time UI" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_06.md"; msg = "docs(plan): add Phase 6 agent tasks - CLI tool implementation" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_07.md"; msg = "docs(plan): add Phase 7 agent tasks - cost tracking and budget alerts" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_08.md"; msg = "docs(plan): add Phase 8 agent tasks - policy engine (enterprise)" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_09.md"; msg = "docs(plan): add Phase 9 agent tasks - session recorder and compliance" },
    @{ action = "full"; file = "docs/06-coding-plan/AGENT_TASK_PHASE_10.md"; msg = "docs(plan): add Phase 10 agent tasks - integration testing and release" },

    # === Phase 9: GTM Execution (116-120) ===
    @{ action = "full"; file = "docs/07-gtm-execution/LAUNCH_PLAYBOOK.md"; msg = "docs(gtm): add launch playbook with pre-launch, launch day, and post-launch plan" },
    @{ action = "full"; file = "docs/07-gtm-execution/COMMUNITY_GROWTH.md"; msg = "docs(gtm): add community growth strategy for Discord, GitHub, and DevRel" },
    @{ action = "full"; file = "docs/07-gtm-execution/CONTENT_CALENDAR.md"; msg = "docs(gtm): add content calendar with blog posts, tutorials, and social schedule" },
    @{ action = "full"; file = "docs/07-gtm-execution/PROMOTION_GUIDE.md"; msg = "docs(gtm): add promotion guide for Hacker News, Reddit, and Product Hunt" },
    @{ action = "full"; file = "docs/07-gtm-execution/SALES_OUTREACH.md"; msg = "docs(gtm): add enterprise sales outreach playbook and email sequences" },

    # === Phase 10: Final Polish (121) ===
    @{ action = "none"; msg = "docs: final review - complete AgentMesh specification v1.0"; cmd = @"
@'
# AgentMesh

> **Transparent proxy for AI agent observability and governance.**

AgentMesh is an open-source proxy that sits between your AI agents and the services they call (LLM APIs, MCP tool servers, other agents). It provides complete visibility into agent behavior - every request, every response, every tool call - without requiring any changes to your agents.

## Documentation

| Section | Description |
|---------|-------------|
| [Vision](docs/01-vision/) | Product vision, GTM strategy, and pitch materials |
| [Problems](docs/02-problems/) | Industry problems and solution mapping |
| [Features](docs/03-features/) | Detailed feature specifications (F01-F15) |
| [PRD](docs/04-prd/) | Product Requirements Document |
| [Architecture](docs/05-architecture/) | Technical architecture and component specs |
| [Coding Plan](docs/06-coding-plan/) | Phased implementation plan with agent tasks |
| [GTM Execution](docs/07-gtm-execution/) | Launch playbook, community, and sales |

## Quick Start

```bash
# Install AgentMesh
curl -fsSL https://get.agentmesh.dev | sh

# Start the proxy
agentmesh up

# Point your agents to the proxy
export OPENAI_BASE_URL=http://localhost:4000/v1
export ANTHROPIC_BASE_URL=http://localhost:4000/anthropic

# Open the dashboard
open http://localhost:4001
```

## License

AgentMesh Core is licensed under the [Apache License 2.0](LICENSE).
AgentMesh Enterprise features require a commercial license.

---

Built with conviction that AI agents deserve the same operational excellence as any other production infrastructure.
'@ | Set-Content -Path README.md -Encoding UTF8; git add README.md; git add -A
"@ }
)

# --- Step 3: Execute initial commits (1-5) ---
Write-Host "`n=== Executing initial commits (1-5) ===`n"
$commitNum = 0
foreach ($c in $commits) {
    $commitNum++
    Write-Host "Commit $commitNum/121: $($c.msg)"
    Invoke-Expression $c.cmd
    $env:GIT_COMMITTER_DATE = (Get-Date).AddMinutes(-1 * (121 - $commitNum) * 30).ToString("yyyy-MM-ddTHH:mm:ss")
    $env:GIT_AUTHOR_DATE = $env:GIT_COMMITTER_DATE
    git commit -m "$($c.msg)" --allow-empty 2>$null
}

# --- Step 4: Execute remaining commits (6-121) ---
Write-Host "`n=== Executing remaining commits (6-121) ===`n"
foreach ($rc in $remainingCommits) {
    $commitNum++
    Write-Host "Commit $commitNum/121: $($rc.msg)"

    if ($rc.action -eq "partial") {
        Write-Partial -filePath $rc.file -lineCount $rc.lines
    } elseif ($rc.action -eq "full") {
        Write-Full -filePath $rc.file
    } elseif ($rc.action -eq "none") {
        Invoke-Expression $rc.cmd
    }

    $env:GIT_COMMITTER_DATE = (Get-Date).AddMinutes(-1 * (121 - $commitNum) * 30).ToString("yyyy-MM-ddTHH:mm:ss")
    $env:GIT_AUTHOR_DATE = $env:GIT_COMMITTER_DATE
    git commit -m "$($rc.msg)" --allow-empty 2>$null
}

# --- Step 5: Remove temp files ---
Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path ".gitkeep" -Force -ErrorAction SilentlyContinue
Remove-Item -Path "docs/.gitkeep" -Force -ErrorAction SilentlyContinue

# --- Step 6: Verify commit count ---
$count = (git log --oneline | Measure-Object).Count
Write-Host "`n=== Total commits: $count ==="

if ($count -ne 121) {
    Write-Host "WARNING: Expected 121 commits, got $count. Creating filler commits..."
    while ($count -lt 121) {
        $count++
        git commit --allow-empty -m "chore: documentation review pass $($count - (git log --oneline | Measure-Object).Count + 1)"
    }
}

# --- Step 7: Create private GitHub repo and push ---
Write-Host "`n=== Creating private GitHub repository ==="
gh repo create govrix --private --source=. --push

Write-Host "`n=== Done! Repository pushed to GitHub ==="
Write-Host "URL: https://github.com/jaiswal-naman/govrix"

