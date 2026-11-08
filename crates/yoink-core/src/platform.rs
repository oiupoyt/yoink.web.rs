use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Platform {
    pub key: String,
    pub label: String,
}

pub fn detect_platform(url: &str) -> Platform {
    let lower = url.to_lowercase();
    let host = if let Some(start) = lower.find("://") {
        let rest = &lower[start + 3..];
        rest.split(['/', '?', '#', ':']).next().unwrap_or("")
    } else {
        lower.split(['/', '?', '#', ':']).next().unwrap_or("")
    };

    let check = |domains: &[&str]| domains.iter().any(|&d| host == d || host.ends_with(&format!(".{}", d)));

    if check(&["youtube.com", "youtu.be", "music.youtube.com"]) {
        Platform {
            key: "youtube".into(),
            label: "YouTube".into(),
        }
    } else if check(&["x.com", "twitter.com"]) {
        Platform {
            key: "x".into(),
            label: "X / Twitter".into(),
        }
    } else if check(&["instagram.com"]) {
        Platform {
            key: "instagram".into(),
            label: "Instagram".into(),
        }
    } else if check(&["threads.net", "threads.com"]) {
        Platform {
            key: "threads".into(),
            label: "Threads".into(),
        }
    } else if check(&["tiktok.com"]) {
        Platform {
            key: "tiktok".into(),
            label: "TikTok".into(),
        }
    } else if check(&["vimeo.com"]) {
        Platform {
            key: "vimeo".into(),
            label: "Vimeo".into(),
        }
    } else if check(&["twitch.tv"]) {
        Platform {
            key: "twitch".into(),
            label: "Twitch".into(),
        }
    } else if check(&["reddit.com"]) {
        Platform {
            key: "reddit".into(),
            label: "Reddit".into(),
        }
    } else if check(&["facebook.com", "fb.watch"]) {
        Platform {
            key: "facebook".into(),
            label: "Facebook".into(),
        }
    } else if host.is_empty() {
        Platform {
            key: "unknown".into(),
            label: "Unknown site".into(),
        }
    } else {
        Platform {
            key: "generic".into(),
            label: host.to_string(),
        }
    }
}

pub fn is_probably_url(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return !trimmed.contains(char::is_whitespace);
    }
    false
}
