use crate::bus::BusState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use gestalt_core::application::agent::registry::{AgentEntry, AgentRegistry};
use gestalt_core::application::agent::xavier::XavierClient;
use gestalt_router::{
    agent::SubprocessRunner,
    event_bus::BusEvent,
    router::Router as GestaltRouter,
    run::{AgentSpec, RunSpec},
};
use gestalt_state::memstate::MemState;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Drain timeout honoured by the HTTP server, sourced from
/// `GESTALT_SHUTDOWN_TIMEOUT_MS` (default 5s), clamped to 10 minutes.
fn drain_timeout_from_env() -> Duration {
    std::env::var("GESTALT_SHUTDOWN_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(|ms| Duration::from_millis(ms.min(600_000)))
        .unwrap_or(Duration::from_secs(5))
}

struct ActiveRunEntry {
    router: Arc<GestaltRouter>,
    join_handle: tokio::task::JoinHandle<()>,
}

static ACTIVE_RUNS: OnceLock<std::sync::Mutex<HashMap<usize, ActiveRunEntry>>> = OnceLock::new();
static NEXT_RUN_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

fn active_runs() -> &'static std::sync::Mutex<HashMap<usize, ActiveRunEntry>> {
    ACTIVE_RUNS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

/// Finalize and cancel all in-flight Task API orchestrations on server shutdown concurrently,
/// bounded by a given timeout deadline to ensure bounded overall shutdown.
pub async fn finalize_active_task_runs(timeout: Option<Duration>) {
    let entries: Vec<ActiveRunEntry> = {
        let mut guard = active_runs().lock().unwrap();
        guard.drain().map(|(_, v)| v).collect()
    };

    if entries.is_empty() {
        return;
    }

    // Finalize all active runs concurrently: abort the outer task, await its termination,
    // and then cancel the router so all state records and worktrees are cleanly finalized.
    // Default to 8 seconds if not specified (covers the agent task drain + worktree cleanup with headroom),
    // clamped to any provided remaining shutdown budget.
    let sync_start = std::time::Instant::now();
    // 1. Immediately abort the outer orchestration join handles so they stop dispatching new work
    for entry in &entries {
        entry.join_handle.abort();
    }

    // 2. Immediate synchronous phase: mark cancellation and persist terminal crashed/cancelled states
    // in StateDb (SQLite) and MemState immediately for every entry. This guarantees run states are terminal
    // and cannot be lost even if the remaining timeout is zero or very short.
    for entry in &entries {
        entry.router.cancel_current_run_sync();
    }

    // 3. Graceful asynchronous phase: await in-flight task termination, perform racing-write reconciliation,
    // and cleanup worktrees within the provided remaining shutdown budget (discounting the time taken by sync persistence).
    let raw_budget = timeout.unwrap_or(Duration::from_secs(8));
    let cancel_budget = raw_budget.saturating_sub(sync_start.elapsed());
    if cancel_budget.is_zero() {
        tracing::warn!("finalize_active_task_runs: shutdown budget exhausted; terminal state persisted, skipping task drain");
        return;
    }

    let cancel_all_fut = async {
        let mut join_set = tokio::task::JoinSet::new();
        for entry in entries {
            let r = entry.router.clone();
            let handle = entry.join_handle;
            join_set.spawn(async move {
                let _ = handle.await;
                r.cancel_current_run().await;
            });
        }
        while join_set.join_next().await.is_some() {}
    };

    if tokio::time::timeout(cancel_budget, cancel_all_fut)
        .await
        .is_err()
    {
        tracing::warn!(
            "finalize_active_task_runs: concurrent cancellation timed out after {:?}",
            cancel_budget
        );
    }
}

static GLOBAL_MEM_STATE: OnceLock<MemState> = OnceLock::new();

pub fn get_mem_state() -> MemState {
    GLOBAL_MEM_STATE.get_or_init(MemState::new).clone()
}

#[derive(serde::Deserialize)]
pub struct TaskRequest {
    pub task: String,
    pub capabilities: Vec<String>,
    pub max_parallel: Option<usize>,
}

/// Router for the declarative task api.
pub fn router() -> Router<BusState> {
    Router::new()
        .route("/api/task", post(handle_task_http))
        .merge(crate::xavier_plugin::router())
}

/// Future that resolves when the process should stop accepting connections.
///
/// Kept as a boxed future (instead of `crate::shutdown::ShutdownCoordinator`)
/// so this module stays self-contained: integration tests include this file
/// directly with `#[path]`, where `crate::shutdown` does not exist.
pub type ShutdownFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

/// Run the combined bus and task API HTTP server on `host:port`.
pub async fn serve(
    host: &str,
    port: u16,
    db_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    serve_with_shutdown(host, port, db_path, None).await
}

/// Run the bus + task API server, draining in-flight connections when
/// `shutdown` resolves (`axum`'s graceful shutdown still answers any request
/// already being served).
pub async fn serve_with_shutdown(
    host: &str,
    port: u16,
    db_path: Option<&str>,
    shutdown: Option<ShutdownFuture>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state_db_path = db_path.map(std::path::PathBuf::from).unwrap_or_else(|| {
        home::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".gestalt")
            .join("state.db")
    });

    let db = Arc::new(gestalt_state::StateDb::open(&state_db_path)?);

    let sink = std::env::var("XAVIER_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
        .map(|_| Arc::new(gestalt_router::xavier_sink::XavierEventSink::from_env()));

    if let Some(ref s) = sink {
        if s.is_available().await {
            tracing::info!("Bus → Xavier sink connected (Xavier :8006)");
        } else {
            tracing::warn!("Xavier not reachable — events will queue in StateDb (replay later)");
        }
    } else {
        tracing::warn!("XAVIER_TOKEN not set — bus running WITHOUT Xavier streaming");
    }

    let bus_state = crate::bus::BusState {
        db: db.clone(),
        sink,
    };

    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();

    // Wrap app in a cancellation middleware so in-flight requests abort if drain expires
    let app = crate::bus::build_router(bus_state.clone())
        .merge(router().with_state(bus_state))
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let token = token_clone.clone();
            async move {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        (
                            StatusCode::SERVICE_UNAVAILABLE,
                            "Server shutdown drain timeout expired",
                        )
                            .into_response()
                    }
                    res = next.run(req) => res,
                }
            }
        }));

    let addr = format!("{}:{}", host, port);

    tracing::info!("Gestalt Event Bus + Task API listening on http://{addr}");
    println!("📡 Gestalt Event Bus + Task API on http://{addr}");
    println!("   POST /api/event  — push a BusEvent");
    println!("   GET  /api/events — tail recent events");
    println!("   POST /api/task   — declarative routing task API");
    println!("   GET  /healthz    — liveness");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    match shutdown {
        Some(wait) => {
            // Signal graceful shutdown when `wait` resolves, then bound the remaining drain
            // to GESTALT_SHUTDOWN_TIMEOUT_MS so a stalled request cannot hang indefinitely.
            let drain_deadline = drain_timeout_from_env();
            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

            let mut server_task = tokio::spawn(async move {
                axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        let _ = shutdown_rx.await;
                    })
                    .await
            });

            let mut drain_start: Option<std::time::Instant> = None;
            let select_res = tokio::select! {
                res = &mut server_task => {
                    match res {
                        Ok(inner) => inner.map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                    }
                }
                _ = wait => {
                    drain_start = Some(std::time::Instant::now());
                    let _ = shutdown_tx.send(());
                    // Axum is now draining in the background. Enforce drain_deadline.
                    match tokio::time::timeout(drain_deadline, &mut server_task).await {
                        Ok(res) => {
                            match res {
                                Ok(inner) => inner.map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                            }
                        }
                        Err(_) => {
                            tracing::warn!(
                                "Event Bus + Task API graceful drain exceeded {:?}; abandoning",
                                drain_deadline
                            );
                            cancel_token.cancel();
                            server_task.abort();
                            let _ = server_task.await;
                            Ok(())
                        }
                    }
                }
            };
            tracing::info!("Event Bus + Task API stopped accepting connections (drained)");
            // Pass the remaining shutdown budget so total shutdown (HTTP drain + task finalization)
            // shares and respects GESTALT_SHUTDOWN_TIMEOUT_MS.
            let remaining_budget = match drain_start {
                Some(start) => drain_deadline.saturating_sub(start.elapsed()),
                None => drain_deadline,
            };
            finalize_active_task_runs(Some(remaining_budget)).await;
            select_res?;
        },
        None => {
            axum::serve(listener, app).await?;
        },
    }

    Ok(())
}

/// Handler for POST /api/task
async fn handle_task_http(
    State(bus_state): State<BusState>,
    Json(req): Json<TaskRequest>,
) -> impl IntoResponse {
    let registry_path = AgentRegistry::resolve_path("agent-registry.toml");
    let registry = match AgentRegistry::load(&registry_path) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to load agent registry: {}", e)
                })),
            )
                .into_response();
        },
    };

    // Filter agents by capability intersection
    let matching_agents: Vec<AgentEntry> = registry
        .agents
        .iter()
        .filter(|agent| {
            req.capabilities
                .iter()
                .all(|req_cap| agent.capabilities.contains(req_cap))
        })
        .cloned()
        .collect();

    if matching_agents.is_empty() {
        let available: Vec<serde_json::Value> = registry
            .agents
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "capabilities": a.capabilities,
                })
            })
            .collect();

        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "status": "error",
                "message": "No matching agents found for requested capabilities",
                "requested_capabilities": req.capabilities,
                "available_agents": available,
            })),
        )
            .into_response();
    }

    let run_id = uuid::Uuid::new_v4();
    let agent_ids: Vec<String> = matching_agents.iter().map(|a| a.name.clone()).collect();

    // Emit run_started event on the bus (traceability)
    let ev = BusEvent::new(
        "gestalt",
        "run_started",
        format!("Task API orchestration started: {}", req.task),
    )
    .with_run_id(run_id.to_string())
    .with_state("Running")
    .with_metadata(serde_json::json!({
        "agents": agent_ids,
        "task": req.task,
        "requested_by": "task_api",
    }));

    let db_clone = bus_state.db.clone();
    let sink_clone = bus_state.sink.clone();
    tokio::spawn(async move {
        let _ =
            gestalt_router::event_bus::handle_event(&db_clone, &ev, sink_clone.as_deref()).await;
    });

    // Wire matching agents to RunSpec and launch execution asynchronously
    let agent_specs: Vec<AgentSpec> = matching_agents
        .iter()
        .map(|a| AgentSpec {
            id: a.name.clone(),
            command: a.name.clone(),
            args: Vec::new(),
            allowed_paths: None,
            env: None,
            capabilities: a.capabilities.clone(),
        })
        .collect();

    let max_parallel = req.max_parallel.unwrap_or(4);
    let run_spec = RunSpec {
        base_ref: "main".to_string(),
        task: req.task.clone(),
        agents: agent_specs,
        max_parallel,
        timeout: 300,
        push: false,
        integration_branch: None,
    };

    let db = bus_state.db.clone();
    let runner = SubprocessRunner::new(Duration::from_secs(300));
    let router_inst = GestaltRouter::new(None, Arc::new(runner), db, get_mem_state(), None, None);
    let router_arc = Arc::new(router_inst);
    let run_key = NEXT_RUN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Use a notify channel so the task does not execute or remove its entry before registration completes
    let (start_tx, start_rx) = tokio::sync::oneshot::channel::<()>();

    let router_for_spawn = router_arc.clone();
    let handle = tokio::spawn(async move {
        let _ = start_rx.await;
        let xavier_client = XavierClient::from_env();
        if xavier_client.is_available().await {
            router_for_spawn.set_xavier(xavier_client);
        }
        let _ = router_for_spawn.execute(run_spec).await;
        active_runs().lock().unwrap().remove(&run_key);
    });

    {
        active_runs().lock().unwrap().insert(
            run_key,
            ActiveRunEntry {
                router: router_arc,
                join_handle: handle,
            },
        );
    }
    let _ = start_tx.send(());

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "run_id": run_id.to_string(),
            "agents": agent_ids,
        })),
    )
        .into_response()
}
