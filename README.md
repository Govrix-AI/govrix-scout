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

`ash
# Install AgentMesh
curl -fsSL https://get.agentmesh.dev | sh

# Start the proxy
agentmesh up

# Point your agents to the proxy
export OPENAI_BASE_URL=http://localhost:4000/v1
export ANTHROPIC_BASE_URL=http://localhost:4000/anthropic

# Open the dashboard
open http://localhost:4001
`

## License

AgentMesh Core is licensed under the [Apache License 2.0](LICENSE).
AgentMesh Enterprise features require a commercial license.

---

Built with conviction that AI agents deserve the same operational excellence as any other production infrastructure.
