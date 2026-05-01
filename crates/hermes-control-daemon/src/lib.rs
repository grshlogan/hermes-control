use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

pub fn build_router() -> Router {
    Router::new().route("/v1/status", get(status))
}

async fn status() -> Json<Value> {
    Json(json!({
        "status": "skeleton",
        "daemon": "hermes-control-daemon",
        "phase": 1
    }))
}
