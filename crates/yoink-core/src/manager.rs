use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::process::Command;

use crate::error::YoinkError;

const RELEASE_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const MAX_UPDATE_INTERVAL_HOURS: u64 = 30 * 24; // 30 days max

#[derive(Serialize, Deserialize)]
struct UpdateMeta {
    last_check_epoch_secs: u64,
}

pub struct YtDlpManager {
    cache_dir: PathBuf,
    update_interval: Duration,
}

impl YtDlpManager {
    pub fn new() -> Option<Self> {
        Self::with_interval_hours(24)
    }

    pub fn with_interval_hours(hours: u64) -> Option<Self> {
        let effective_hours = hours.min(MAX_UPDATE_INTERVAL_HOURS);
        let dirs = ProjectDirs::from("", "", "yoink")?;
        Some(Self {
            cache_dir: dirs.cache_dir().join("bin"),
            update_interval: Duration::from_secs(effective_hours * 3600),
        })
    }

    pub fn get_cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub async fn ensure_binary(&self) -> Result<PathBuf, YoinkError> {
        let local = self.local_binary_path();

        // Check if an update is due
        let update_due = self.is_update_due().await;

        if update_due {
            // Attempt update/download in background or inline
            match self.download_latest().await {
                Ok(path) => return Ok(path),
                Err(e) => {
                    // If update failed (e.g. offline), fall back to existing local or PATH
                    if self.is_executable(&local).await {
                        return Ok(local);
                    }
                    if self.is_in_path().await {
                        return Ok(PathBuf::from("yt-dlp"));
                    }
                    return Err(e);
                }
            }
        }

        if self.is_executable(&local).await {
            return Ok(local);
        }

        if self.is_in_path().await {
            return Ok(PathBuf::from("yt-dlp"));
        }

        // Neither local nor PATH exists, download required
        self.download_latest().await
    }

    pub async fn is_in_path(&self) -> bool {
        Command::new("yt-dlp")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub async fn is_executable(&self, path: &Path) -> bool {
        Command::new(path)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn local_binary_path(&self) -> PathBuf {
        let bin_name = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        };
        self.cache_dir.join(bin_name)
    }

    async fn is_update_due(&self) -> bool {
        let meta_path = self.cache_dir.join("last_check.json");
        let content = match tokio::fs::read_to_string(&meta_path).await {
            Ok(c) => c,
            Err(_) => return true,
        };

        let meta: UpdateMeta = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(_) => return true,
        };

        let now_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now_epoch.saturating_sub(meta.last_check_epoch_secs) >= self.update_interval.as_secs()
    }

    pub async fn download_latest(&self) -> Result<PathBuf, YoinkError> {
        tokio::fs::create_dir_all(&self.cache_dir).await?;

        let asset_name = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else if cfg!(target_os = "macos") {
            "yt-dlp_macos"
        } else if cfg!(target_arch = "aarch64") {
            "yt-dlp_linux_aarch64"
        } else {
            "yt-dlp_linux"
        };

        let url = format!("{}/{}", RELEASE_BASE, asset_name);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| YoinkError::YtDlpCommandError(e.to_string()))?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| YoinkError::YtDlpCommandError(format!("Failed to download yt-dlp: {}", e)))?;

        if !resp.status().is_success() {
            return Err(YoinkError::YtDlpCommandError(format!(
                "Failed to download yt-dlp: HTTP {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| YoinkError::YtDlpCommandError(e.to_string()))?;

        let dest = self.local_binary_path();
        let tmp_dest = dest.with_extension("download");

        tokio::fs::write(&tmp_dest, bytes).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&tmp_dest).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&tmp_dest, perms).await?;
        }

        tokio::fs::rename(&tmp_dest, &dest).await?;

        let meta = UpdateMeta {
            last_check_epoch_secs: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        let _ = tokio::fs::write(
            self.cache_dir.join("last_check.json"),
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        )
        .await;

        Ok(dest)
    }
}
