pub mod manifest;

use axum::{routing::get, Json, Router};
use manifest::XavierPluginManifest;

async fn get_manifest() -> Json<XavierPluginManifest> {
    Json(XavierPluginManifest::default())
}

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/api/plugin/manifest", get(get_manifest))
        .layer(axum::middleware::from_fn(crate::bus::auth_middleware))
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
}

#[cfg(test)]
mod tests;
