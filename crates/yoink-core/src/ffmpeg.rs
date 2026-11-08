use std::path::PathBuf;
use tokio::process::Command;

pub async fn find_ffmpeg() -> Option<PathBuf> {
    let success = Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if success {
        Some(PathBuf::from("ffmpeg"))
    } else {
        None
    }
}
