use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use crate::error::YoinkError;
use crate::format::{build_choices, VideoInfo};
use crate::platform::detect_platform;
use crate::traits::ProbeResult;

pub async fn run_probe(ytdlp: &Path, url: &str) -> Result<ProbeResult, YoinkError> {
    let mut cmd_args = vec!["-J".to_string(), "--no-warnings".to_string()];
    cmd_args.extend(crate::args_helper::build_common_ytdlp_args(None));
    cmd_args.push(url.to_string());

    let output = Command::new(ytdlp)
        .args(&cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(YoinkError::YtDlpCommandError(clean_ytdlp_error(&stderr)));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let raw_val: serde_json::Value = serde_json::from_str(&stdout_str)?;

    let is_playlist = raw_val.get("_type").and_then(|t| t.as_str()) == Some("playlist")
        || raw_val.get("entries").and_then(|e| e.as_array()).is_some();

    let mut info: VideoInfo = serde_json::from_value(raw_val)?;
    info.is_playlist = is_playlist;

    let info_json_path = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let p = std::env::temp_dir().join(format!("yoink-info-{}-{}.json", std::process::id(), now));
        if tokio::fs::write(&p, stdout_str.as_bytes()).await.is_ok() {
            Some(p)
        } else {
            None
        }
    };

    let choices = build_choices(&info);
    let platform = detect_platform(url);

    Ok(ProbeResult {
        info,
        choices,
        info_json_path,
        platform,
        is_playlist,
    })
}

pub fn clean_ytdlp_error(stderr: &str) -> String {
    for line in stderr.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("ERROR:") {
            let err = trimmed.trim_start_matches("ERROR:").trim();
            if let Some(pos) = err.find(']') {
                return err[pos + 1..].trim().to_string();
            }
            return err.to_string();
        }
    }
    let last = stderr.lines().last().unwrap_or("Unknown error").trim();
    if last.is_empty() {
        "yt-dlp exited with an error".to_string()
    } else {
        last.to_string()
    }
}
