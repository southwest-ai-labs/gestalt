#!/usr/bin/env python3
"""
Dispatch Ola 2 of Gestalt to Google Jules API.
Dispatches 15 decoupled tasks with isolated file islands to saturate the iberi22 account slots.
"""

import os
import sys
import json
import time
import glob
import urllib.request
import urllib.error
from pathlib import Path

GESTALT_DIR = Path(__file__).resolve().parent.parent
BODIES_DIR = GESTALT_DIR / ".hermes" / "gestalt-ola2"
WAVE_OUT_DIR = GESTALT_DIR / ".gitcore" / "waves" / "ola-v2-gestalt"
WAVE_OUT_FILE = WAVE_OUT_DIR / "dispatched_sessions.json"
BUS_URL = "http://127.0.0.1:8081/api/event"
ENV_FILE = Path.home() / ".hermes" / ".env"
JULES_API_BASE = "https://jules.googleapis.com/v1alpha"

def log(msg):
    print(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {msg}", flush=True)

def emit_bus_event(agent, event_type, summary, data=None):
    try:
        payload = {
            "agent": agent,
            "event_type": event_type,
            "summary": summary,
            "data": data or {}
        }
        req = urllib.request.Request(
            BUS_URL,
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            res = json.loads(resp.read().decode())
            log(f"Bus Event emitted: seq={res.get('seq')}")
    except Exception as e:
        log(f"Bus Event warning: {e}")

def get_jules_key():
    if not ENV_FILE.exists():
        log(f"Error: {ENV_FILE} not found")
        sys.exit(1)
    for line in ENV_FILE.read_text().splitlines():
        line = line.strip()
        if line.startswith("JULES_API_KEY="):
            return line.split("=", 1)[1].strip("\"'")
    log("Error: JULES_API_KEY not found in ~/.hermes/.env")
    sys.exit(1)

def get_recent_sessions(key):
    req = urllib.request.Request(
        f"{JULES_API_BASE}/sessions?pageSize=50",
        headers={"X-Goog-Api-Key": key}
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode())
            return data.get("sessions", [])
    except Exception as e:
        log(f"Warning fetching existing sessions: {e}")
        return []

def main():
    WAVE_OUT_DIR.mkdir(parents=True, exist_ok=True)
    key = get_jules_key()
    log("Found Jules API key for iberi22 account.")

    body_files = sorted(glob.glob(str(BODIES_DIR / "body-*.md")))
    if not body_files:
        log(f"No body files found in {BODIES_DIR}")
        sys.exit(1)

    log(f"Found {len(body_files)} task bodies to dispatch.")

    existing_sessions = get_recent_sessions(key)
    existing_titles = {s.get("title"): s.get("id") for s in existing_sessions if s.get("title")}

    tracked = []
    if WAVE_OUT_FILE.exists():
        try:
            tracked = json.loads(WAVE_OUT_FILE.read_text())
        except Exception:
            tracked = []

    tracked_by_tid = {item["task_id"]: item for item in tracked}

    for bpath in body_files:
        p = Path(bpath)
        tid = f"W2-{p.stem.replace('body-', '')}"
        content = p.read_text()
        first_line = content.splitlines()[0]
        title = first_line.lstrip("# ").strip()

        if tid in tracked_by_tid and tracked_by_tid[tid].get("session_id"):
            log(f"Task {tid} already in wave ledger: session {tracked_by_tid[tid]['session_id']}. Skipping.")
            continue

        if title in existing_titles:
            sid = existing_titles[title]
            log(f"Task {tid} matches active/recent session {sid}. Re-linking.")
            item = {
                "task_id": tid,
                "session_id": sid,
                "title": title,
                "integrated": False
            }
            tracked_by_tid[tid] = item
            tracked.append(item)
            continue

        log(f"Dispatching {tid}: '{title}' to Jules...")
        payload = {
            "prompt": content,
            "sourceContext": {
                "source": "sources/github/southwest-ai-labs/gestalt",
                "githubRepoContext": {
                    "startingBranch": "main"
                }
            }
        }

        req = urllib.request.Request(
            f"{JULES_API_BASE}/sessions",
            data=json.dumps(payload).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "X-Goog-Api-Key": key
            }
        )

        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                res = json.loads(resp.read().decode())
                sid = res.get("id")
                log(f"✅ Successfully created session {sid} for {tid}")
                item = {
                    "task_id": tid,
                    "session_id": sid,
                    "title": title,
                    "integrated": False
                }
                tracked.append(item)
                tracked_by_tid[tid] = item

                emit_bus_event(
                    "jules-account-iberi22",
                    "wave_dispatched",
                    f"Dispatched {tid} to Jules (session {sid}): {title}",
                    {"task_id": tid, "session_id": sid, "wave": "ola-v2-gestalt"}
                )

                WAVE_OUT_FILE.write_text(json.dumps(tracked, indent=2))
                time.sleep(2)  # Avoid burst rate limits
        except Exception as e:
            log(f"❌ Error creating session for {tid}: {e}")

    WAVE_OUT_FILE.write_text(json.dumps(tracked, indent=2))
    log(f"=== Wave 2 Dispatch Completed. Total tracked tasks: {len(tracked)} ===")

if __name__ == "__main__":
    main()
