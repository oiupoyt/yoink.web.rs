use crate::error::YoinkError;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct YtDlpManager {
    cache_dir: PathBuf,
}

impl YtDlpManager {
    pub fn new() -> Option<Self> {
        let dirs = ProjectDirs::from("", "", "yoink")?;
        Some(Self {
            cache_dir: dirs.cache_dir().join("bin"),
        })
    }

    pub async fn ensure_binary(&self) -> Result<PathBuf, YoinkError> {
        // 1. Check PATH
        if self.is_in_path().await {
            return Ok(PathBuf::from("yt-dlp"));
        }

        // 2. Check cache
        let local = self.cache_dir.join("yt-dlp");
        if self.is_executable(&local).await {
            return Ok(local);
        }

        // TODO: Self-update and download logic
        Err(YoinkError::YtDlpMissing)
    }

    async fn is_in_path(&self) -> bool {
        Command::new("yt-dlp")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    async fn is_executable(&self, path: &Path) -> bool {
        Command::new(path)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }
}
