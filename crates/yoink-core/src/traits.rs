use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::error::YoinkError;
use crate::format::{DownloadChoice, VideoInfo};
use crate::platform::Platform;
use crate::progress::ProgressEvent;

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub info: VideoInfo,
    pub choices: Vec<DownloadChoice>,
    pub info_json_path: Option<PathBuf>,
    pub platform: Platform,
    pub is_playlist: bool,
}

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub choice: DownloadChoice,
    pub out_dir: PathBuf,
    pub info_json_path: Option<PathBuf>,
}

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn probe(&self, url: &str) -> Result<ProbeResult, YoinkError>;
    async fn download(
        &self,
        req: DownloadRequest,
    ) -> Result<(broadcast::Receiver<ProgressEvent>, JoinHandle<Result<PathBuf, YoinkError>>), YoinkError>;
}
