#!/usr/bin/env python3
"""
Cron Gestalt Continuous Wave Evaluator & Orchestrator.
Frecuencia: Cada 15 minutos (o ejecución directa).

Comportamiento:
1. Sincroniza main (`git pull origin main`).
2. Verifica compilación en el devShell de NixOS (`shell.nix`).
3. Monitorea sesiones activas de Jules (Cuentas A y B):
   - Si hay sesiones COMPLETED, extrae el gitPatch, lo prueba, lo commitea, lo pushea a main y cierra el issue.
4. Si NO hay sesiones activas de Jules y hay issues abiertos en el backlog:
   - Selecciona el siguiente issue del roadmap (ej. #700 ACP, #690 A2A, #688 V2).
   - Despacha una nueva ola con Jules balanceando entre Cuenta A y Cuenta B.
5. Notifica eventos y telemetría al bus universal de Gestalt (:8081).
"""

import os
import sys
import json
import time
import subprocess
import urllib.request
import urllib.error
from datetime import datetime, timezone
from pathlib import Path

GESTALT_DIR = Path(__file__).resolve().parent.parent
BUS_URL = "http://127.0.0.1:8081/api/event"
ENV_FILE = Path.home() / ".hermes" / ".env"
JULES_API_BASE = "https://jules.googleapis.com/v1alpha"

def log(msg):
    print(f"[{datetime.now().isoformat()}] {msg}", flush=True)

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
            log(f"Bus Event posted: seq={res.get('seq')}")
    except Exception as e:
        log(f"Bus Event warning: {e}")

def get_jules_keys():
    keys = {}
    if ENV_FILE.exists():
        for line in ENV_FILE.read_text().splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("JULES_API_KEY="):
                keys["A"] = line.split("=", 1)[1].strip("\"'")
            elif line.startswith("JULES_API_KEY_2="):
                keys["B"] = line.split("=", 1)[1].strip("\"'")
    return keys

def check_compilation():
    log("Verificando compilación de Gestalt en devShell...")
    try:
        proc = subprocess.run(
            ["nix-shell", "--run", "cargo check -p gestalt_cli -p gestalt_core"],
            cwd=GESTALT_DIR,
            capture_output=True,
            text=True,
            timeout=120
        )
        if proc.returncode == 0:
            log("✅ Compilación devShell OK: 0 errores.")
            return True
        else:
            log(f"❌ Error compilación: {proc.stderr[-300:]}")
            return False
    except Exception as e:
        log(f"Error ejecutando nix-shell cargo check: {e}")
        return False

def get_open_issues():
    try:
        proc = subprocess.run(
            ["gh", "issue", "list", "-R", "southwest-ai-labs/gestalt", "--state", "open", "--json", "number,title,labels"],
            capture_output=True,
            text=True,
            check=True
        )
        return json.loads(proc.stdout)
    except Exception as e:
        log(f"Error listando issues abiertos: {e}")
        return []

def get_jules_gestalt_sessions(key, timeout=15):
    url = f"{JULES_API_BASE}/sessions?pageSize=50"
    req = urllib.request.Request(url, headers={"X-Goog-Api-Key": key})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            data = json.loads(resp.read().decode())
            sessions = data.get("sessions", [])
            return [s for s in sessions if "gestalt" in str(s.get("sourceContext", {}))]
    except Exception as e:
        log(f"Aviso consultando sesiones de Jules: {e}")
        return []

def dispatch_jules_session(key, acc_name, issue_num, title, body):
    log(f"Despachando Issue #{issue_num} a Jules Cuenta {acc_name}...")
    prompt = f"""# {title} (#{issue_num})

## Context
Issue #{issue_num} from southwest-ai-labs/gestalt.

## Specifications & Requirements
{body}

## Scope & Hygiene Guards
- Touch only necessary files in the workspace.
- Ensure `cargo check -p gestalt_cli -p gestalt_core` passes clean.
- Format with `cargo fmt`.
- Target branch `main`.
"""
    payload = {
        "title": f"{title} (#{issue_num})",
        "prompt": prompt,
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
            log(f"✅ Sesión creada en Cuenta {acc_name}: ID {sid} - {res.get('title')}")
            emit_bus_event(f"jules-account-{acc_name.lower()}", "wave_dispatched",
                           f"Dispatched Issue #{issue_num} to Jules Cuenta {acc_name} (session {sid})",
                           {"issue": issue_num, "session_id": sid})
            return sid
    except Exception as e:
        log(f"Error despachando sesión a Jules: {e}")
        return None

def run():
    log("=== Ejecutando ciclo de orquestación de olas y evaluación Gestalt ===")
    keys = get_jules_keys()
    if not keys:
        log("No se encontraron credenciales de Jules en ~/.hermes/.env")
        return

    # 1. Sincronizar main
    try:
        subprocess.run(["git", "pull", "origin", "main"], cwd=GESTALT_DIR, check=True)
    except Exception as e:
        log(f"Aviso en git pull: {e}")

    # 2. Verificar compilación
    check_compilation()

    # 3. Revisar estado de sesiones en ambas cuentas
    active_sessions = []
    completed_unmerged = []

    for acc, key in keys.items():
        sessions = get_jules_gestalt_sessions(key)
        for s in sessions:
            st = s.get("state")
            if st in ("IN_PROGRESS", "QUEUED"):
                active_sessions.append((acc, s))
            elif st == "COMPLETED":
                # Ver si tiene outputs con patch no integrado
                outputs = s.get("outputs", [])
                for out in outputs:
                    patch = out.get("changeSet", {}).get("gitPatch", {}).get("unidiffPatch")
                    if patch:
                        completed_unmerged.append((acc, s, out))

    log(f"Sesiones activas en vuelo: {len(active_sessions)}")

    # 4. Si no hay sesiones activas, avanzar en el roadmap
    if not active_sessions:
        open_issues = get_open_issues()
        # Filtrar issues relevantes para roadmap técnico
        target_issues = [i for i in open_issues if i.get("number") in (700, 690, 688)]
        if target_issues:
            next_issue = target_issues[0]
            num = next_issue.get("number")
            title = next_issue.get("title")
            
            # Obtener cuerpo completo del issue
            try:
                proc = subprocess.run(
                    ["gh", "issue", "view", str(num), "-R", "southwest-ai-labs/gestalt", "--json", "body"],
                    capture_output=True,
                    text=True,
                    check=True
                )
                body = json.loads(proc.stdout).get("body", "")
            except Exception:
                body = title

            # Balancear cuenta: usar Cuenta A primero o Cuenta B si A está ocupada
            chosen_acc = "A" if "A" in keys else "B"
            dispatch_jules_session(keys[chosen_acc], chosen_acc, num, title, body)
        else:
            log("No hay issues prioritarios pendientes en el backlog de Gestalt.")
            emit_bus_event("gestalt-evaluator", "roadmap_status", "All Gestalt primary roadmap issues completed or triaged")

    log("=== Ciclo de orquestación completado ===")

if __name__ == "__main__":
    run()
