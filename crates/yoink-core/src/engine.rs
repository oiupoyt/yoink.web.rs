use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::error::YoinkError;
use crate::ffmpeg::find_ffmpeg;
use crate::manager::YtDlpManager;
use crate::probe::{clean_ytdlp_error, run_probe};
use crate::progress::{parse_progress_line, ProgressEvent, PROGRESS_PREFIX, PROGRESS_TEMPLATE};
use crate::traits::{DownloadRequest, Downloader, ProbeResult};

pub struct YtDlpEngine {
    manager: YtDlpManager,
}

impl YtDlpEngine {
    pub async fn new() -> Result<Self, YoinkError> {
        let manager = YtDlpManager::new().ok_or(YoinkError::YtDlpMissing)?;
        Ok(Self { manager })
    }
}

#[async_trait]
impl Downloader for YtDlpEngine {
    async fn probe(&self, url: &str) -> Result<ProbeResult, YoinkError> {
        let ytdlp = self.manager.ensure_binary().await?;
        run_probe(&ytdlp, url).await
    }

    async fn download(
        &self,
        req: DownloadRequest,
    ) -> Result<(broadcast::Receiver<ProgressEvent>, JoinHandle<Result<PathBuf, YoinkError>>), YoinkError> {
        let ytdlp = self.manager.ensure_binary().await?;
        let (tx, rx) = broadcast::channel(64);

        let handle = tokio::spawn(async move {
            let mut args = Vec::new();
            if let Some(json_path) = &req.info_json_path {
                args.push("--load-info-json".to_string());
                args.push(json_path.to_string_lossy().to_string());
            } else {
                args.push(req.url.clone());
            }

            for arg in &req.choice.args {
                args.push(arg.clone());
            }

            args.extend([
                "--no-playlist".to_string(),
                "--no-warnings".to_string(),
                "--newline".to_string(),
                "--no-quiet".to_string(),
                "--progress".to_string(),
                "--progress-template".to_string(),
                format!("download:{}", PROGRESS_TEMPLATE),
                "--print".to_string(),
                "after_move:filepath".to_string(),
                "--no-simulate".to_string(),
                "-o".to_string(),
                req.out_dir.join("%(title).60s.%(ext)s").to_string_lossy().to_string(),
            ]);

            if let Some(ffmpeg) = find_ffmpeg().await {
                args.push("--ffmpeg-location".to_string());
                args.push(ffmpeg.to_string_lossy().to_string());
            }

            let _ = tx.send(ProgressEvent::Starting);

            let mut child = Command::new(&ytdlp)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let stdout = child.stdout.take().ok_or_else(|| {
                YoinkError::YtDlpCommandError("Failed to capture stdout of yt-dlp".to_string())
            })?;
            let stderr = child.stderr.take().ok_or_else(|| {
                YoinkError::YtDlpCommandError("Failed to capture stderr of yt-dlp".to_string())
            })?;

            let mut reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            let mut final_filepath: Option<PathBuf> = None;
            let mut destinations = Vec::new();
            let mut part: u32 = 0;
            let mut total_parts: u32 = 1;
            let mut last_downloaded: u64 = 0;

            let stderr_handle = tokio::spawn(async move {
                let mut err_lines = Vec::new();
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    err_lines.push(line);
                }
                err_lines.join("\n")
            });

            while let Ok(Some(line)) = reader.next_line().await {
                let line_str = line.trim();
                if line_str.is_empty() {
                    continue;
                }

                if line_str.contains(PROGRESS_PREFIX) {
                    if let Some(event) = parse_progress_line(line_str, part, total_parts) {
                        if let ProgressEvent::Downloading { downloaded_bytes, .. } = &event {
                            if *downloaded_bytes < last_downloaded {
                                part += 1;
                            }
                            last_downloaded = *downloaded_bytes;
                        }
                        let _ = tx.send(event);
                    }
                } else if line_str.contains("Downloading 1 format(s):") {
                    if let Some(part_str) = line_str.split("format(s):").nth(1) {
                        let count = part_str.trim().split('+').count();
                        if count > 0 {
                            total_parts = count as u32;
                        }
                    }
                } else if line_str.contains("[Merger]") || line_str.contains("[ExtractAudio]") {
                    let step = if line_str.contains("[Merger]") {
                        "merging formats"
                    } else {
                        "extracting audio"
                    };
                    let _ = tx.send(ProgressEvent::Processing { step: step.into() });
                } else if let Some(dest) = line_str.strip_prefix("[download] Destination: ") {
                    destinations.push(PathBuf::from(dest.trim()));
                } else if Path::new(line_str).is_absolute() {
                    final_filepath = Some(PathBuf::from(line_str));
                }
            }

            let status = child.wait().await?;
            let stderr_output = stderr_handle.await.unwrap_or_default();

            if !status.success() {
                cleanup_partial_files(&destinations).await;
                let clean_err = clean_ytdlp_error(&stderr_output);
                let _ = tx.send(ProgressEvent::Error {
                    message: clean_err.clone(),
                });
                return Err(YoinkError::YtDlpCommandError(clean_err));
            }

            if let Some(final_path) = final_filepath {
                let _ = tx.send(ProgressEvent::Finished {
                    filepath: final_path.clone(),
                });
                Ok(final_path)
            } else {
                Err(YoinkError::YtDlpCommandError("Could not determine downloaded file path".to_string()))
            }
        });

        Ok((rx, handle))
    }
}

async fn cleanup_partial_files(paths: &[PathBuf]) {
    for p in paths {
        let _ = tokio::fs::remove_file(p).await;
        let _ = tokio::fs::remove_file(format!("{}.part", p.display())).await;
        let _ = tokio::fs::remove_file(format!("{}.ytdl", p.display())).await;
    }
}
