use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

use crate::error::YoinkError;
use crate::format::{DownloadChoice, VideoInfo};
use crate::platform::detect_platform;
use crate::progress::ProgressEvent;
use crate::traits::{DownloadRequest, Downloader, ProbeResult};

pub struct MockDownloader {
    pub title: String,
    pub delay_ms: u64,
}

impl MockDownloader {
    pub fn new(title: String) -> Self {
        Self { title, delay_ms: 10 }
    }
}

#[async_trait]
impl Downloader for MockDownloader {
    async fn probe(&self, url: &str) -> Result<ProbeResult, YoinkError> {
        let platform = detect_platform(url);
        let info = VideoInfo {
            title: self.title.clone(),
            uploader: Some("Mock Channel".into()),
            duration: Some(180.0),
            webpage_url: Some(url.to_string()),
            extractor_key: Some(platform.key.clone()),
            formats: None,
            is_playlist: false,
            entries: None,
        };

        let choices = vec![
            DownloadChoice {
                label: "1080p · mp4 · ~25 MB".into(),
                kind: "video".into(),
                args: vec!["-f".into(), "bestvideo+bestaudio".into()],
            },
            DownloadChoice {
                label: "720p · mp4 · ~12 MB".into(),
                kind: "video".into(),
                args: vec!["-f".into(), "bestvideo[height<=720]+bestaudio".into()],
            },
            DownloadChoice {
                label: "audio only · mp3 · ~3 MB".into(),
                kind: "audio".into(),
                args: vec!["-x".into(), "--audio-format".into(), "mp3".into()],
            },
        ];

        Ok(ProbeResult {
            info,
            choices,
            info_json_path: None,
            platform,
            is_playlist: false,
        })
    }

    async fn download(
        &self,
        req: DownloadRequest,
    ) -> Result<(broadcast::Receiver<ProgressEvent>, JoinHandle<Result<PathBuf, YoinkError>>), YoinkError> {
        let (tx, rx) = broadcast::channel(32);
        let delay = self.delay_ms;
        let title = self.title.clone();
        let out_dir = req.out_dir;

        let handle = tokio::spawn(async move {
            let total = 25_000_000u64;
            let steps = 5;
            for i in 1..=steps {
                sleep(Duration::from_millis(delay)).await;
                let downloaded = total * i / steps;
                let _ = tx.send(ProgressEvent::Downloading {
                    downloaded_bytes: downloaded,
                    total_bytes: Some(total),
                    speed_bps: Some(5_000_000),
                    eta_seconds: Some((steps - i) * delay / 1000),
                    part: 0,
                    total_parts: 1,
                });
            }

            let _ = tx.send(ProgressEvent::Processing {
                step: "merging formats".into(),
            });
            sleep(Duration::from_millis(delay)).await;

            tokio::fs::create_dir_all(&out_dir).await?;
            let filename = format!("{}.mp4", title.replace(['/', '\\', ' '], "_"));
            let dest = out_dir.join(filename);
            tokio::fs::write(&dest, b"mock media content").await?;

            let _ = tx.send(ProgressEvent::Finished {
                filepath: dest.clone(),
            });

            Ok(dest)
        });

        Ok((rx, handle))
    }
}
