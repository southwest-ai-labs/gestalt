#!/usr/bin/env python3
"""
Generador y Despachador de la Ola Completa Gestalt v1.0 (14 Tareas Disjuntas).
Sigue el template canónico de gitcore-jules-issues (ACs medibles, file islands disjuntos, PR delivery requirements).
Despacha las 14 tareas a la cuenta principal 'iberi22' (JULES_API_KEY).
"""

import os
import json
import time
import urllib.request
from pathlib import Path

def get_jules_key():
    env_file = Path.home() / ".hermes" / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("JULES_API_KEY="):
                return line.split("=", 1)[1].strip("\"'")
    return os.environ.get("JULES_API_KEY", "")

KEY = get_jules_key()
REPO_SOURCE = "sources/github/southwest-ai-labs/gestalt"
API_URL = "https://jules.googleapis.com/v1alpha/sessions"

tasks = [
    {
        "id": "W01",
        "title": "feat(a2a): implement Agent Card endpoint and A2A protocol schema definitions",
        "file_island": "gestalt_core/src/adapters/a2a/",
        "goal": "Implement Google A2A protocol Agent Card endpoint (/.well-known/agent.json) and basic task payload schemas in gestalt_core.",
        "files": ["gestalt_core/src/adapters/a2a/mod.rs", "gestalt_core/src/adapters/a2a/agent_card.rs", "gestalt_core/src/adapters/a2a/types.rs"],
        "do_not_touch": ["gestalt_mcp/**", "gestalt-router/**"]
    },
    {
        "id": "W02",
        "title": "feat(a2a): implement inbound A2A task dispatcher and execution state mapping",
        "file_island": "gestalt_core/src/application/a2a/",
        "goal": "Implement A2A task runner mapping submitted, working, completed, failed, and canceled states to Gestalt agent runs.",
        "files": ["gestalt_core/src/application/a2a/mod.rs", "gestalt_core/src/application/a2a/dispatcher.rs"],
        "do_not_touch": ["gestalt_mcp/**", "gestalt-state/**"]
    },
    {
        "id": "W03",
        "title": "feat(observe): add automated hook injector for Claude and OpenCode sessions",
        "file_island": "gestalt_cli/src/observe/injectors/",
        "goal": "Implement dedicated hook auto-injection functions for Claude Code (~/.claude/settings.json) and OpenCode (~/.config/opencode/hooks.json).",
        "files": ["gestalt_cli/src/observe/injectors/mod.rs", "gestalt_cli/src/observe/injectors/claude_hook.rs", "gestalt_cli/src/observe/injectors/opencode_hook.rs"],
        "do_not_touch": ["gestalt_cli/src/main.rs", "gestalt-router/**"]
    },
    {
        "id": "W04",
        "title": "feat(observe): add automated hook injector for Hermes and AgY sessions",
        "file_island": "gestalt_cli/src/observe/injectors/hermes_agy/",
        "goal": "Implement hook auto-injection functions for Hermes agent gateway and AgY CLI to track session starts/stops.",
        "files": ["gestalt_cli/src/observe/injectors/hermes_hook.rs", "gestalt_cli/src/observe/injectors/agy_hook.rs"],
        "do_not_touch": ["gestalt_cli/src/main.rs", "gestalt-ws/**"]
    },
    {
        "id": "W05",
        "title": "feat(cli): add 'gestalt observe install-hooks' command for global telemetry",
        "file_island": "gestalt_cli/src/commands/observe_install.rs",
        "goal": "Add CLI command `gestalt observe install-hooks` to run all discovered agent hook injectors with dry-run support.",
        "files": ["gestalt_cli/src/commands/observe_install.rs", "gestalt_cli/src/commands/mod.rs"],
        "do_not_touch": ["gestalt_core/**", "gestalt_mcp/**"]
    },
    {
        "id": "W06",
        "title": "feat(plugin): implement Xavier Plugin Manifest and status endpoint in task_api",
        "file_island": "gestalt_cli/src/xavier_plugin/",
        "goal": "Expose GET /api/plugin/manifest returning Gestalt plugin metadata compatible with Xavier PluginsManager.",
        "files": ["gestalt_cli/src/xavier_plugin/mod.rs", "gestalt_cli/src/xavier_plugin/manifest.rs"],
        "do_not_touch": ["gestalt-state/**", "gestalt-search/**"]
    },
    {
        "id": "W07",
        "title": "feat(bus): add event filter and pagination query parameters to GET /api/events",
        "file_island": "gestalt_cli/src/api_routes/event_query.rs",
        "goal": "Support `agent`, `event_type`, `since_seq`, and `limit` query parameters on the Gestalt event bus tail endpoint.",
        "files": ["gestalt_cli/src/api_routes/mod.rs", "gestalt_cli/src/api_routes/event_query.rs"],
        "do_not_touch": ["gestalt_core/**", "gestalt_mcp/**"]
    },
    {
        "id": "W08",
        "title": "feat(statedb): add pruning policy and retention limit for historical bus events",
        "file_island": "gestalt-state/src/retention.rs",
        "goal": "Add configurable SQLite event table pruning keeping the latest 50,000 events or 30 days of telemetry.",
        "files": ["gestalt-state/src/retention.rs", "gestalt-state/tests/retention_test.rs"],
        "do_not_touch": ["gestalt-router/**", "gestalt-search/**"]
    },
    {
        "id": "W09",
        "title": "feat(mcp): add tool 'gestalt_bus_query' to gestalt_mcp server",
        "file_island": "gestalt_mcp/src/tools/bus_query.rs",
        "goal": "Expose an MCP tool `gestalt_bus_query` allowing LLM agents to query recent mesh events and agent activities.",
        "files": ["gestalt_mcp/src/tools/bus_query.rs", "gestalt_mcp/src/tools/mod.rs"],
        "do_not_touch": ["gestalt_cli/**", "gestalt-state/**"]
    },
    {
        "id": "W10",
        "title": "feat(mcp): add tool 'gestalt_agent_status' to query local running CLIs",
        "file_island": "gestalt_mcp/src/tools/agent_status.rs",
        "goal": "Expose an MCP tool `gestalt_agent_status` that returns real-time process state and version of installed agent CLIs.",
        "files": ["gestalt_mcp/src/tools/agent_status.rs"],
        "do_not_touch": ["gestalt_core/**", "gestalt-ws/**"]
    },
    {
        "id": "W11",
        "title": "feat(search): add tag-based filtering and relevance boosting to hybrid search",
        "file_island": "gestalt-search/src/tag_filter.rs",
        "goal": "Enhance hybrid Tantivy BM25 + vector search to filter and boost results by tags like [skill], [memory], or [code].",
        "files": ["gestalt-search/src/tag_filter.rs", "gestalt-search/tests/tag_filter_test.rs"],
        "do_not_touch": ["gestalt_cli/**", "gestalt_mcp/**"]
    },
    {
        "id": "W12",
        "title": "feat(ws): add channel subscription filter to WebSocket event stream",
        "file_island": "gestalt-ws/src/filter.rs",
        "goal": "Allow WebSocket clients to subscribe to specific event types or agents (e.g. `subscribe:agent=jules`).",
        "files": ["gestalt-ws/src/filter.rs", "gestalt-ws/tests/filter_test.rs"],
        "do_not_touch": ["gestalt_core/**", "gestalt-state/**"]
    },
    {
        "id": "W13",
        "title": "test(e2e): add end-to-end multi-agent mesh bus integration test suite",
        "file_island": "gestalt_cli/tests/e2e_mesh_bus.rs",
        "goal": "Add an E2E integration test suite simulating concurrent event emission and deduplication across 4 synthetic agents.",
        "files": ["gestalt_cli/tests/e2e_mesh_bus.rs"],
        "do_not_touch": ["gestalt_core/**", "gestalt_mcp/**", "gestalt-router/**"]
    },
    {
        "id": "W14",
        "title": "docs(protocol): create comprehensive standalone and plugin integration manual",
        "file_island": "docs/protocol/GESTALT_PLUGIN_STANDALONE_SPEC.md",
        "goal": "Document the complete architectural specification for running Gestalt standalone or as a Xavier plugin.",
        "files": ["docs/protocol/GESTALT_PLUGIN_STANDALONE_SPEC.md", "README.md"],
        "do_not_touch": ["*.rs", "Cargo.toml", "Cargo.lock"]
    }
]

def make_body(t):
    return f"""# {t['title']}

## Context
Wave 1 Gestalt Stabilization & Ecosystem Plugin Integration — Task {t['id']}.
Target repository: `southwest-ai-labs/gestalt`, starting from branch `main`.

## Current State (MEASURABLE)
- File island: `{t['file_island']}`.
- Specific deliverables: {', '.join(t['files'])}.
- Isolation guarantee: No file collision with other tasks in the wave.

## Desired State (DELTA)
{t['goal']}

## Acceptance Criteria (VERIFIABLE BY COMMAND)
- [ ] `git status --porcelain` lists files strictly within `{t['file_island']}` (or docs for documentation tasks).
- [ ] `cargo check -p gestalt_cli -p gestalt_core` exits 0 (or workspace check).
- [ ] `cargo fmt -- --check` passes with zero formatting issues.
- [ ] Unit/Integration tests added for the new capability pass 100% green.

## Files to Modify (File Island)
{chr(10).join(f"- `{f}` (LOW risk)" for f in t['files'])}

## DO NOT Touch
{chr(10).join(f"- `{d}`" for d in t['do_not_touch'])}

## PR Delivery Requirements (ANTI-EMPTY-PR)
- [ ] `git status --porcelain` shows modified/untracked files before opening PR.
- [ ] `git diff --stat HEAD` lists files (NOT empty).
- [ ] Open Pull Request targeting `main`.
"""

print(f"Preparing to dispatch {len(tasks)} tasks to Jules account (iberi22)...")

dispatched = []
for t in tasks:
    body = make_body(t)
    payload = {
        "title": f"[{t['id']}] {t['title']}",
        "prompt": body,
        "sourceContext": {
            "source": REPO_SOURCE,
            "githubRepoContext": {
                "startingBranch": "main"
            }
        }
    }
    req = urllib.request.Request(
        API_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "X-Goog-Api-Key": KEY
        }
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode())
            sid = data.get("id")
            title = data.get("title")
            print(f"✅ Dispatched [{t['id']}] -> Session ID: {sid} ({title[:60]})")
            dispatched.append({"task_id": t["id"], "session_id": sid, "title": title})
            time.sleep(1.5) # Evitar rate limits
    except Exception as e:
        print(f"❌ Failed to dispatch [{t['id']}]: {e}")

out_path = Path(__file__).resolve().parent.parent / ".gitcore" / "waves" / "ola-v1-gestalt" / "dispatched_sessions.json"
out_path.write_text(json.dumps(dispatched, indent=2))
print(f"\nAll dispatched sessions recorded to {out_path}")
