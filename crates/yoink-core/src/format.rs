use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFormat {
    pub format_id: String,
    pub ext: Option<String>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub abr: Option<f64>,
    pub tbr: Option<f64>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub webpage_url: Option<String>,
    pub extractor_key: Option<String>,
    pub formats: Option<Vec<RawFormat>>,
    #[serde(default)]
    pub is_playlist: bool,
    pub entries: Option<Vec<VideoInfo>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadChoice {
    pub label: String,
    pub kind: String, // "video" or "audio"
    pub args: Vec<String>,
}

const MAX_VIDEO_CHOICES: usize = 8;

pub fn build_choices(info: &VideoInfo) -> Vec<DownloadChoice> {
    let formats = info.formats.as_deref().unwrap_or(&[]);
    let mut choices = Vec::new();

    let audio_only: Vec<&RawFormat> = formats
        .iter()
        .filter(|f| {
            let acodec = f.acodec.as_deref().unwrap_or("none");
            let vcodec = f.vcodec.as_deref().unwrap_or("none");
            acodec != "none" && vcodec == "none"
        })
        .collect();

    let best_audio = {
        let mut sorted = audio_only.clone();
        sorted.sort_by(|a, b| {
            let br_a = a.abr.or(a.tbr).unwrap_or(0.0);
            let br_b = b.abr.or(b.tbr).unwrap_or(0.0);
            br_b.partial_cmp(&br_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.first().copied()
    };

    let audio_size = best_audio.and_then(|a| a.filesize.or(a.filesize_approx)).unwrap_or(0);

    let videos: Vec<&RawFormat> = formats
        .iter()
        .filter(|f| {
            let vcodec = f.vcodec.as_deref().unwrap_or("none");
            vcodec != "none" && f.height.is_some() && f.height.unwrap() > 0
        })
        .collect();

    let mut heights_set = BTreeSet::new();
    for v in &videos {
        if let Some(h) = v.height {
            heights_set.insert(h);
        }
    }
    let mut heights: Vec<u32> = heights_set.into_iter().collect();
    heights.reverse(); // highest to lowest

    for &h in heights.iter().take(MAX_VIDEO_CHOICES) {
        let candidates: Vec<&RawFormat> = videos.iter().filter(|v| v.height == Some(h)).copied().collect();
        let mut sorted = candidates;
        sorted.sort_by(|a, b| score_video(b).cmp(&score_video(a)));
        if let Some(best) = sorted.first() {
            let muxed = best.acodec.as_deref().unwrap_or("none") != "none";
            let v_size = best.filesize.or(best.filesize_approx).unwrap_or(0);
            let size = v_size + if muxed { 0 } else { audio_size };
            let size_label = if size > 0 {
                format!(" · ~{}", format_bytes(size))
            } else {
                String::new()
            };

            choices.push(DownloadChoice {
                kind: "video".to_string(),
                label: format!("{}p · mp4{}", h, size_label),
                args: vec![
                    "-f".to_string(),
                    format!("bv*[height={}]+ba/b[height={}]/bv*[height<={}] +ba/b", h, h, h).replace(' ', ""),
                    "--merge-output-format".to_string(),
                    "mp4".to_string(),
                ],
            });
        }
    }

    if choices.is_empty() {
        choices.push(DownloadChoice {
            kind: "video".to_string(),
            label: "best available · mp4".to_string(),
            args: vec![
                "-f".to_string(),
                "bv*+ba/b".to_string(),
                "--merge-output-format".to_string(),
                "mp4".to_string(),
            ],
        });
    }

    let audio_label = if audio_size > 0 {
        format!("audio only · mp3 · ~{}", format_bytes(audio_size))
    } else {
        "audio only · mp3".to_string()
    };

    choices.push(DownloadChoice {
        kind: "audio".to_string(),
        label: audio_label,
        args: vec![
            "-f".to_string(),
            "ba/b".to_string(),
            "-x".to_string(),
            "--audio-format".to_string(),
            "mp3".to_string(),
            "--audio-quality".to_string(),
            "0".to_string(),
        ],
    });

    choices
}

fn score_video(f: &RawFormat) -> i64 {
    let mut score = (f.tbr.unwrap_or(0.0)) as i64;
    if f.ext.as_deref() == Some("mp4") {
        score += 10_000;
    }
    if f.vcodec.as_deref().map(|c| c.starts_with("avc")).unwrap_or(false) {
        score += 5_000;
    }
    score
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut val = bytes as f64;
    let mut unit_idx = 0;
    while val >= 1024.0 && unit_idx < units.len() - 1 {
        val /= 1024.0;
        unit_idx += 1;
    }
    if val >= 10.0 || unit_idx == 0 {
        format!("{} {}", val.round() as u64, units[unit_idx])
    } else {
        format!("{:.1} {}", val, units[unit_idx])
    }
}

pub fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "".to_string();
    }
    let s = seconds.round() as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, sec)
    } else {
        format!("{}:{:02}", m, sec)
    }
}

pub fn format_speed(bps: u64) -> String {
    if bps == 0 {
        String::new()
    } else {
        format!("{}/s", format_bytes(bps))
    }
}

pub fn format_eta(seconds: u64) -> String {
    format_duration(seconds as f64)
}

pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() > max {
        let truncated: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        text.to_string()
    }
}

pub fn shorten_path(filepath: &str, homedir: &str, max: usize) -> String {
    let pretty = if filepath.starts_with(homedir) {
        format!("~{}", &filepath[homedir.len()..])
    } else {
        filepath.to_string()
    };
    if pretty.chars().count() <= max {
        pretty
    } else {
        let ext = filepath.rsplit('.').next().map(|e| format!(".{}", e)).unwrap_or_default();
        let avail = max.saturating_sub(ext.len() + 1);
        let prefix: String = pretty.chars().take(avail).collect();
        format!("{}…{}", prefix, ext)
    }
}

pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
