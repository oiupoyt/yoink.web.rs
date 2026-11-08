use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/jobs", post(create_job_handler));

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
