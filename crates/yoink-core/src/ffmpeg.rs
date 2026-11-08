use std::path::PathBuf;
use tokio::process::Command;

pub async fn find_ffmpeg() -> Option<PathBuf> {
    // If ffmpeg is in PATH, yt-dlp locates it automatically; return None so we don't pass invalid relative arg.
    let in_path = Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if in_path {
        // Check if which/whereis returns absolute path
        if let Ok(output) = Command::new("which").arg("ffmpeg").output().await {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
        return None;
    }

    None
}
