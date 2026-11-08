---
name: gestalt-agent-mesh
description: Skill guidelines for integrating and operating within the Gestalt multi-agent SWAL mesh.
triggers:
  - Gestalt integration
  - Event emission
  - Task execution
  - Worktree leases
---

# Gestalt Agent Mesh Skill

This document provides structured guidelines and architectural blueprints for AI agents (Hermes, Claude, Antigravity, OpenCode) interacting with the Gestalt multi-agent Software Workspace Agent Layer (SWAL) mesh. It covers endpoint usage, event emission, task delegation, run ID correlation, lease acquisition, and memory synergy with Xavier.

## Core Architectural Overview

Gestalt acts as the local orchestrator and synchronization hub for an autonomous mesh of specialized AI tools. Agents must explicitly register their state changes and activities with Gestalt to maintain an updated `timeline` table in the local SQLite `StateDb`. By strictly adhering to these operational protocols, agents minimize conflicting file operations and achieve higher-order observability.

## API Endpoints

Gestalt exposes several core HTTP endpoints that agents should use. Detailed curl examples are provided in `references/api_reference.md`.

### 1. `/healthz`
**Purpose:** Ensure the Gestalt instance is reachable and healthy before initiating complex mesh operations.
**Method:** `GET`
**Description:** A simple health check returning standard success (200 OK) if the internal event bus, database, and router are operational. If this endpoint returns 5xx or connection refused, agents should fallback to offline standalone modes.

### 2. `/api/health/providers`
**Purpose:** Verify the availability of specific underlying AI providers configured for Gestalt.
**Method:** `GET`
**Description:** Essential for sub-agent delegations. Agents querying this can dynamically check which providers (e.g., `claude`, `opencode-go`, `muse-spark-1.2-contributor`) are available and green in the local configuration. 

### 3. `/api/event`
**Purpose:** Emit state changes, logs, and activity telemetry to the Gestalt Event Bus.
**Method:** `POST`
**Description:** The primary ingress point for the event timeline. All payloads must adhere to the `BusEvent` schema and include appropriate run ID correlation headers or payload properties. This is a fire-and-forget endpoint that enables Gestalt to syndicate events to WebSocket clients.

### 4. `/api/events`
**Purpose:** Query historical events from the Gestalt timeline.
**Method:** `GET`
**Description:** Used for retrieving recent mesh activity. Supports query parameters like `since_seq` (cursor pagination), `project`, and `agent` filtering. Critical for agent context gathering—understanding what other agents have recently executed before taking action.

### 5. `/api/task`
**Purpose:** Delegate or register tasks within the mesh.
**Method:** `POST`
**Description:** Used in A2A (Agent-to-Agent) scenarios. An agent can post a task payload here, which Gestalt's `Dispatcher` will manage. The payload tracks the task through `TaskState` (Submitted, Working, Completed, Failed, Canceled).

## Protocol Guidelines

### Run ID Correlation
When an agent starts a sequence of actions (e.g., a "run" or "session"), it must generate or inherit a unique Run ID (UUIDv4). This Run ID **must** be attached to every event emitted via `/api/event`. This allows Gestalt and viewing tools (like `gestalt-tui.sh`) to group parallel streams of events cleanly.
- Inject Run ID in event payload: `"run_id": "uuid-here"`

### Event Emission Schemas
All events posted to `/api/event` should ideally map to the `AgentEvent` enum (`StepProgress`, `ToolInvoked`, `ErrorCaptured`) tagged with a `snake_case` `event_type`.
- Minimal schema structure:
  ```json
  {
    "agent": "your-agent-name",
    "event_type": "tool_invoked",
    "project": "my-repo",
    "payload": {
      "tool": "read_file",
      "args": {"path": "src/main.rs"}
    }
  }
  ```

### Lease Acquisition for Concurrent Writes
In a multi-agent environment, modifying files concurrently leads to severe Git conflicts and corrupted states. Agents must practice strict lease acquisition before modifying source files:
1. **Acquire:** Request a file modification lease from Gestalt or coordinate via standard branch checks.
2. **Execute:** Perform the write or unified diff BlockEdit.
3. **Commit/Release:** Ensure the changes are committed and the lease is implicitly or explicitly released.
4. If an overlapping file modification occurs, Gestalt's `gestalt-router` merge queue (`integrate_branches`) will attempt to resolve it or cleanly roll back the tree.

### Xavier Memory Synergy
Xavier functions as the RTK Proxy and archival memory system. Agents should seamlessly integrate their local executions with Xavier's remote state.
1. **Execution Summary:** At the end of an execution run, post an execution summary memory (`kind=run_result`) to Xavier's archival endpoint.
2. **Context Injection:** When initialized, agents should ingest `XAVIER_CONTEXT` and `SEARCH_CONTEXT` provided by Gestalt to augment their baseline prompts.
3. **Routing:** Sub-agent commands can be routed via `xavier exec` or `xavier_run_command` when Xavier is responsive, falling back to direct shell execution when offline while emitting `[xavier-proxy]` telemetry.
