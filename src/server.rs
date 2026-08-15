use std::sync::{Arc, RwLock};

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::state::{Event, SharedState};

#[derive(Deserialize)]
pub struct EventsQuery {
    after: Option<usize>,
}

async fn state_handler(
    State(shared): State<Arc<RwLock<SharedState>>>,
) -> impl IntoResponse {
    let snapshot = shared.read().unwrap().snapshot();
    Json(snapshot)
}

async fn events_handler(
    State(shared): State<Arc<RwLock<SharedState>>>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    let after = q.after.unwrap_or(0);
    let events: Vec<Event> = shared.read().unwrap().events_after(after);
    Json(events)
}

pub async fn run(config: &Config, shared: Arc<RwLock<SharedState>>) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/api/state", get(state_handler))
        .route("/api/events", get(events_handler))
        .fallback_service(ServeDir::new("web"))
        .with_state(shared);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
