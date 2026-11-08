use thiserror::Error;

#[derive(Error, Debug)]
pub enum YoinkError {
    #[error("yt-dlp command failed: {0}")]
    YtDlpCommandError(String),
    #[error("yt-dlp binary missing")]
    YtDlpMissing,
    #[error("ffmpeg binary missing")]
    FfmpegMissing,
    #[error("Download cancelled")]
    Cancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}
