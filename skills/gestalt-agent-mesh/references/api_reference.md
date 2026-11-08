# Gestalt API Reference

This document provides exact `curl` examples and HTTP payload structures for communicating with the Gestalt multi-agent mesh endpoints.

## Base URL
By default, the Gestalt event bus and HTTP router bind to `http://127.0.0.1:8081`. 
Ensure you substitute the port if your local instance is configured differently.

---

## 1. Health Check (`/healthz`)

Verify the Gestalt instance is reachable. (Also aliased as `/health` and `/bus/health`).

### Request
```bash
curl -X GET http://127.0.0.1:8081/healthz
```

### Response (200 OK)
```json
{
  "status": "ok",
  "version": "1.2.0"
}
```

---

## 2. Health Providers (`/api/health/providers`)

Verify the health and availability of configured underlying AI providers.

### Request
```bash
curl -X GET http://127.0.0.1:8081/api/health/providers
```

### Response (200 OK)
```json
{
  "providers": [
    {
      "name": "claude",
      "status": "healthy"
    },
    {
      "name": "opencode-go",
      "status": "healthy"
    }
  ]
}
```

---

## 3. Emit Event (`/api/event`)

Post an event to the Gestalt timeline. Note that aliases like `/bus/event` are also supported.

### Request
```bash
curl -X POST http://127.0.0.1:8081/api/event \
  -H "Content-Type: application/json" \
  -d '{
    "agent": "hermes",
    "event_type": "tool_invoked",
    "project": "core-backend",
    "run_id": "a1b2c3d4-e5f6-7890-abcd-1234567890ab",
    "payload": {
      "tool_name": "list_files",
      "status": "success",
      "duration_ms": 145
    }
  }'
```

### Response (201 Created)
```json
{
  "status": "event_accepted",
  "seq": 14052
}
```

---

## 4. Query Events (`/api/events`)

Query historical events from the timeline, supporting cursor pagination.

### Request
```bash
# Query events for a specific project after sequence 14000
curl -X GET "http://127.0.0.1:8081/api/events?project=core-backend&since_seq=14000&limit=50"
```

### Response (200 OK)
```json
{
  "events": [
    {
      "seq": 14001,
      "agent": "claude",
      "event_type": "step_progress",
      "project": "core-backend",
      "timestamp": "2023-10-27T10:00:00Z",
      "payload": {
        "message": "Analyzing directory structure."
      }
    },
    {
      "seq": 14002,
      "agent": "claude",
      "event_type": "tool_invoked",
      "project": "core-backend",
      "timestamp": "2023-10-27T10:00:05Z",
      "payload": {
        "tool_name": "read_file",
        "file": "src/main.rs"
      }
    }
  ],
  "next_seq": 14002
}
```

---

## 5. Submit Task (`/api/task`)

Submit a new Agent-to-Agent (A2A) task to the Gestalt Dispatcher.

### Request
```bash
curl -X POST http://127.0.0.1:8081/api/task \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": "task-998877",
    "requester": "claude",
    "target_provider": "opencode-go",
    "instruction": "Refactor the authentication middleware in auth.rs",
    "context": {
      "branch": "feat/auth-refactor",
      "lease_acquired": true
    }
  }'
```

### Response (202 Accepted)
```json
{
  "status": "task_submitted",
  "task_id": "task-998877",
  "state": "Submitted"
}
```
