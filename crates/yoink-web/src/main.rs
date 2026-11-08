use axum::{routing::{get, post, delete}, Router, extract::{State, Path}, response::sse::{Event, Sse}};
use std::{net::SocketAddr, sync::Arc, convert::Infallible};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use futures::Stream;

#[derive(Clone)]
struct AppState {
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {});
    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/jobs", post(create_job_handler))
        .route("/api/jobs/:id", get(get_job_handler))
        .route("/api/jobs/:id/events", get(sse_handler))
        .route("/api/jobs/:id", delete(delete_job_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("yoink-web listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> &'static str {
    "{\"status\": \"ok\", \"yt_dlp_version\": \"2026.08.19\", \"ffmpeg_available\": true}"
}

async fn create_job_handler() -> &'static str {
    "{\"job_id\": \"mock-uuid-1234\"}"
}

async fn get_job_handler(Path(id): Path<String>) -> String {
    format!("{{\"job_id\": \"{}\", \"status\": \"queued\"}}", id)
}

async fn delete_job_handler(Path(id): Path<String>) -> String {
    format!("{{\"job_id\": \"{}\", \"status\": \"cancelled\"}}", id)
}

async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, _) = tokio::sync::broadcast::channel::<serde_json::Value>(10);
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        if let Ok(m) = msg {
            Some(Ok(Event::default().data(serde_json::to_string(&m).unwrap())))
        } else {
            None
        }
    });
    Sse::new(stream)
}
