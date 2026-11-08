use axum::{
    body::Body,
    extract::{ConnectInfo, Path, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, post},
    Json, Router,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

use yoink_web::config::Config;
use yoink_web::jobs::{JobManager, JobView};
use yoink_web::sweeper::start_sweeper;
use yoink_web::WebAssets;

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    jobs: Arc<JobManager>,
}

#[derive(Deserialize)]
struct CreateJobRequest {
    url: String,
    format: Option<String>,
    playlist: Option<bool>,
}

#[derive(Serialize)]
struct CreateJobResponse {
    job_id: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    ffmpeg_available: bool,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Arc::new(Config::from_env());
    let jobs = Arc::new(JobManager::new(config.clone()));

    // Start background TTL file cleaner
    start_sweeper(jobs.clone());

    let state = AppState {
        config: config.clone(),
        jobs,
    };

    let api_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/files", get(list_files_handler))
        .route("/jobs", post(create_job_handler))
        .route("/jobs/:id", get(get_job_handler))
        .route("/jobs/:id/events", get(sse_handler))
        .route("/jobs/:id/file", get(file_download_handler))
        .route("/jobs/:id", delete(delete_job_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_and_rate_middleware));

    let app = Router::new()
        .nest("/api", api_routes)
        .fallback(static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let bind_addr: SocketAddr = format!("{}:{}", config.bind, config.port).parse()?;
    println!("yoink-web listening on http://{}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn health_handler() -> Json<HealthResponse> {
    let ffmpeg = yoink_core::ffmpeg::find_ffmpeg().await.is_some();
    Json(HealthResponse {
        status: "ok",
        ffmpeg_available: ffmpeg,
    })
}

async fn list_files_handler(State(state): State<AppState>) -> Json<Vec<JobView>> {
    let files = state.jobs.list_files().await;
    Json(files)
}

async fn create_job_handler(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<CreateJobResponse>), (StatusCode, Json<ErrorResponse>)> {
    let client_ip = get_client_ip(&headers, addr.ip());

    if !state.jobs.check_rate_limit(client_ip).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "Daily download rate limit exceeded".to_string(),
            }),
        ));
    }

    let trimmed_url = req.url.trim().to_string();
    if !yoink_core::platform::is_probably_url(&trimmed_url) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid URL provided".to_string(),
            }),
        ));
    }

    let format = req.format.unwrap_or_else(|| "best".to_string());
    let is_playlist = req.playlist.unwrap_or(false);

    match state
        .jobs
        .create_job(client_ip, trimmed_url, format, is_playlist)
        .await
    {
        Ok(job_id) => Ok((StatusCode::ACCEPTED, Json(CreateJobResponse { job_id }))),
        Err(err) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse { error: err }),
        )),
    }
}

async fn get_job_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobView>, StatusCode> {
    if let Some(view) = state.jobs.get_job(&id).await {
        Ok(Json(view))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn sse_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let rx = state.jobs.get_events(&id).await.ok_or(StatusCode::NOT_FOUND)?;

    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        if let Ok(event) = msg {
            if let Ok(json_str) = serde_json::to_string(&event) {
                return Some(Ok(Event::default().data(json_str)));
            }
        }
        None
    });

    Ok(Sse::new(stream))
}

async fn file_download_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, StatusCode> {
    let (filepath, filename) = state
        .jobs
        .get_job_file(&id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let file = tokio::fs::File::open(&filepath)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mime = mime_guess::from_path(&filename)
        .first_or_octet_stream()
        .to_string();

    let content_disposition = format!("attachment; filename=\"{}\"", filename);

    let resp = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(resp)
}

async fn delete_job_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if state.jobs.cancel_job(&id).await {
        Ok(Json(serde_json::json!({ "deleted": true })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn auth_and_rate_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path();
    let headers = req.headers();
    let client_ip = get_client_ip(headers, addr.ip());

    // 1. IP Allowlist check
    if let Some(allowlist) = &state.config.ip_allowlist {
        let allowed = allowlist.iter().any(|net| net.contains(&client_ip));
        if !allowed {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Forbidden: IP not in allowlist".to_string(),
                }),
            )
                .into_response();
        }
    }

    // 2. Health check is always unauthenticated
    if path == "/api/health" {
        return next.run(req).await;
    }

    // 3. API Key check
    if let Some(expected_key) = &state.config.api_key {
        let auth_header = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));

        let custom_header = headers
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok());

        let key = auth_header.or(custom_header);
        if key != Some(expected_key.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized: Invalid or missing API key".to_string(),
                }),
            )
                .into_response();
        }
    }

    next.run(req).await
}

async fn static_handler(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');
    let file_path = if path.is_empty() { "index.html" } else { path };

    match WebAssets::get(file_path) {
        Some(content) => {
            let mime = mime_guess::from_path(file_path)
                .first_or_octet_stream()
                .as_ref()
                .to_string();

            Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data))
                .unwrap()
        }
        None => match WebAssets::get("index.html") {
            Some(content) => Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(content.data))
                .unwrap(),
            None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
        },
    }
}

fn get_client_ip(headers: &HeaderMap, default_ip: IpAddr) -> IpAddr {
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(first) = xff_str.split(',').next() {
                if let Ok(ip) = first.trim().parse() {
                    return ip;
                }
            }
        }
    }
    default_ip
}
