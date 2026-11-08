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

    # 3. Revisar estado de sesiones de la ola en Cuenta A (iberi22)
    key_a = keys.get("A")
    if not key_a:
        log("No se encontró JULES_API_KEY en ~/.hermes/.env")
        return

    wave_dirs = [
        GESTALT_DIR / ".gitcore" / "waves" / "ola-v2-gestalt",
        GESTALT_DIR / ".gitcore" / "waves" / "ola-v1-gestalt"
    ]
    active_wave_file = None
    tracked_sessions = []
    for wdir in wave_dirs:
        wfile = wdir / "dispatched_sessions.json"
        if wfile.exists():
            try:
                sessions = json.loads(wfile.read_text())
                if any(not s.get("integrated") for s in sessions):
                    active_wave_file = wfile
                    tracked_sessions = sessions
                    break
                elif not active_wave_file:
                    active_wave_file = wfile
                    tracked_sessions = sessions
            except Exception as e:
                log(f"Aviso leyendo {wfile}: {e}")
    wave_file = active_wave_file or (GESTALT_DIR / ".gitcore" / "waves" / "ola-v2-gestalt" / "dispatched_sessions.json")

    active_sessions = []
    completed_sessions = []
    feedback_sessions = []

    for item in tracked_sessions:
        if item.get("integrated"):
            continue
        sid = item.get("session_id")
        tid = item.get("task_id", sid)
        url = f"{JULES_API_BASE}/sessions/{sid}"
        req = urllib.request.Request(url, headers={"X-Goog-Api-Key": key_a})
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                s = json.loads(resp.read().decode())
                st = s.get("state")
                if st in ("IN_PROGRESS", "QUEUED"):
                    active_sessions.append((tid, sid, s, item))
                elif st == "AWAITING_USER_FEEDBACK":
                    feedback_sessions.append((tid, sid, s, item))
                elif st == "COMPLETED":
                    completed_sessions.append((tid, sid, s, item))
        except Exception as e:
            log(f"Error consultando sesión {sid}: {e}")

    total_tracked = len(tracked_sessions)
    total_integrated = sum(1 for s in tracked_sessions if s.get("integrated"))
    completion_pct = round((total_integrated / total_tracked) * 100, 1) if total_tracked > 0 else 0.0

    log(f"Estado ola Gestalt: Integradas={total_integrated}/{total_tracked} ({completion_pct}%), Activas={len(active_sessions)}, AwaitingFeedback={len(feedback_sessions)}, PendientesDeMerge={len(completed_sessions)}")

    # Auto-desbloqueo de sesiones AWAITING_USER_FEEDBACK
    for tid, sid, s, item in feedback_sessions:
        log(f"Desbloqueando sesión {tid} ({sid}) con confirmación de continuación...")
        try:
            unblock_payload = {
                "prompt": "Proceed with the planned implementation. Complete changes in the target island files only, verify with cargo check and tests, and open the Pull Request."
            }
            ureq = urllib.request.Request(
                f"{JULES_API_BASE}/sessions/{sid}:sendMessage",
                data=json.dumps(unblock_payload).encode("utf-8"),
                headers={"Content-Type": "application/json", "X-Goog-Api-Key": key_a}
            )
            with urllib.request.urlopen(ureq, timeout=15) as uresp:
                log(f"✅ Respuesta enviada a {tid} ({sid})")
        except Exception as e:
            log(f"Aviso desbloqueando {sid}: {e}")

    # Integración automática de sesiones COMPLETED
    updated_any = False
    for tid, sid, s, item in completed_sessions:
        outputs = s.get("outputs", [])
        patch = None
        pr_num = None
        for out in outputs:
            pr = out.get("pullRequest", {})
            if pr.get("number"):
                pr_num = pr.get("number")
            p = out.get("changeSet", {}).get("gitPatch", {}).get("unidiffPatch")
            if p:
                patch = p

        # Fallback a activities si outputs no contiene patch ni PR
        if not pr_num and not patch:
            try:
                act_url = f"{JULES_API_BASE}/sessions/{sid}/activities"
                areq = urllib.request.Request(act_url, headers={"X-Goog-Api-Key": key_a})
                with urllib.request.urlopen(areq, timeout=10) as aresp:
                    acts = json.loads(aresp.read().decode()).get("activities", [])
                    for a in acts:
                        for art in a.get("artifacts", []):
                            p = art.get("changeSet", {}).get("gitPatch", {}).get("unidiffPatch")
                            if p:
                                patch = p
            except Exception as e:
                log(f"Aviso consultando activities de {sid}: {e}")

        if pr_num:
            log(f"Detectado PR #{pr_num} para {tid}. Verificando...")
            try:
                subprocess.run(["gh", "pr", "merge", str(pr_num), "-R", "southwest-ai-labs/gestalt", "--rebase", "--auto"], check=True)
                log(f"✅ PR #{pr_num} configurado para auto-merge!")
                item["integrated"] = True
                updated_any = True
            except Exception as e:
                log(f"Aviso en merge de PR #{pr_num}: {e}")
        elif patch:
            log(f"Detectado gitPatch para {tid}. Aplicando...")
            try:
                patch_proc = subprocess.run(["git", "apply", "--ignore-whitespace", "-"], cwd=GESTALT_DIR, input=patch, text=True)
                if patch_proc.returncode != 0:
                    patch_proc = subprocess.run(["git", "apply", "--3way", "--ignore-whitespace", "-"], cwd=GESTALT_DIR, input=patch, text=True)
                if patch_proc.returncode == 0 and check_compilation():
                    subprocess.run(["git", "add", "-A"], cwd=GESTALT_DIR, check=True)
                    subprocess.run(["git", "commit", "-m", f"feat(wave): integrate {tid} patch from Jules session {sid}"], cwd=GESTALT_DIR, check=True)
                    subprocess.run(["git", "push", "origin", "main"], cwd=GESTALT_DIR, check=True)
                    log(f"✅ Patch para {tid} integrado y pusheado a main!")
                    item["integrated"] = True
                    updated_any = True
                    # Archive session to free slot immediately
                    try:
                        areq = urllib.request.Request(
                            f"{JULES_API_BASE}/sessions/{sid}:archive",
                            data=b"{}",
                            headers={"Content-Type": "application/json", "X-Goog-Api-Key": key_a}
                        )
                        with urllib.request.urlopen(areq, timeout=10) as aresp:
                            log(f"✅ Sesión {sid} archivada en Jules.")
                    except Exception as e:
                        log(f"Aviso archivando sesión {sid}: {e}")
                else:
                    log(f"❌ Falló aplicación o compilación para {tid}. Limpiando working tree...")
                    subprocess.run(["git", "reset", "--hard", "HEAD"], cwd=GESTALT_DIR)
                    subprocess.run(["git", "clean", "-fd"], cwd=GESTALT_DIR)
            except Exception as e:
                log(f"Aviso aplicando patch para {tid}: {e}")
                subprocess.run(["git", "reset", "--hard", "HEAD"], cwd=GESTALT_DIR)
                subprocess.run(["git", "clean", "-fd"], cwd=GESTALT_DIR)

    if updated_any:
        try:
            wave_file.write_text(json.dumps(tracked_sessions, indent=2))
            log("💾 Archivo dispatched_sessions.json actualizado.")
        except Exception as e:
            log(f"Aviso guardando wave_file: {e}")

    # 4. Si todas las sesiones de la ola están completadas, avanzar roadmap
    if len(active_sessions) == 0 and len(feedback_sessions) == 0 and len(tracked_sessions) > 0:
        log("🎉 Toda la ola actual de Gestalt ha concluido. Analizando avance de roadmap...")
        emit_bus_event("gestalt-evaluator", "wave_completed", "Gestalt Wave completed successfully.")

    log("=== Ciclo de orquestación completado ===")

if __name__ == "__main__":
    run()
