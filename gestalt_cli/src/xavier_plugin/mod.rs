pub mod manifest;

use axum::{routing::get, Json, Router};
use manifest::XavierPluginManifest;

async fn get_manifest() -> Json<XavierPluginManifest> {
    Json(XavierPluginManifest::default())
}

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/api/plugin/manifest", get(get_manifest))
}

#[cfg(test)]
mod tests;
