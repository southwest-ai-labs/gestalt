use crate::bus::BusState;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct EventsQuery {
    pub agent: Option<String>,
    #[serde(alias = "type")]
    pub event_type: Option<String>,
    pub project: Option<String>,
    pub since_seq: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_events_http(
    State(state): State<BusState>,
    Query(params): Query<EventsQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).clamp(1, 500);
    // Note: the backend uses `after_seq` for pagination cursor, we map `since_seq` query param to `after_seq`.
    let events = state
        .db
        .query_timeline(
            params.agent.as_deref(),
            params.event_type.as_deref(),
            params.project.as_deref(),
            params.since_seq,
            limit,
        )
        .map_err(|e: anyhow::Error| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    let next_seq = events.last().and_then(|e| e.seq);

    Ok(Json(serde_json::json!({
        "count": events.len(),
        "events": events,
        "next_seq": next_seq,
        "cursor": next_seq,
    })))
}
