use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PROGRESS_PREFIX: &str = "YOINK|";
pub const PROGRESS_TEMPLATE: &str = "YOINK|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ProgressEvent {
    Probing {
        status: String,
    },
    Starting,
    Downloading {
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        speed_bps: Option<u64>,
        eta_seconds: Option<u64>,
        part: u32,
        total_parts: u32,
    },
    Processing {
        step: String,
    },
    Finished {
        filepath: PathBuf,
    },
    Error {
        message: String,
    },
}

pub fn parse_progress_line(line: &str, part: u32, total_parts: u32) -> Option<ProgressEvent> {
    let clean = line.trim();
    let payload = if let Some(stripped) = clean.strip_prefix("download:") {
        stripped
    } else {
        clean
    };

    let body = payload.strip_prefix(PROGRESS_PREFIX)?;
    let parts: Vec<&str> = body.split('|').collect();
    if parts.len() < 5 {
        return None;
    }

    let downloaded_bytes = parse_number(parts[0]).unwrap_or(0.0) as u64;
    let total = parse_number(parts[1]).or_else(|| parse_number(parts[2])).map(|v| v as u64);
    let speed_bps = parse_number(parts[3]).map(|v| v as u64);
    let eta_seconds = parse_number(parts[4]).map(|v| v as u64);

    Some(ProgressEvent::Downloading {
        downloaded_bytes,
        total_bytes: total,
        speed_bps,
        eta_seconds,
        part,
        total_parts,
    })
}

fn parse_number(val: &str) -> Option<f64> {
    let v = val.trim();
    if v.is_empty() || v == "None" || v == "NA" {
        None
    } else {
        v.parse::<f64>().ok().filter(|n| n.is_finite())
    }
}
