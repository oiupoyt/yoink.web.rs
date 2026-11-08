use std::path::{Path, PathBuf};

pub fn build_common_ytdlp_args(data_dir: Option<&Path>) -> Vec<String> {
    let mut args = Vec::new();

    // 1. YouTube extractor args: Use tv,mweb,web to bypass datacenter IP bot detection
    let player_clients = std::env::var("YOINK_PLAYER_CLIENT")
        .unwrap_or_else(|_| "tv,mweb,web".to_string());
    args.push("--extractor-args".to_string());
    args.push(format!("youtube:player_client={}", player_clients));

    // 2. JavaScript runtime: modern yt-dlp requires a JS runtime for challenge solving
    if let Some(runtime) = detect_js_runtime() {
        args.push("--js-runtimes".to_string());
        args.push(runtime.to_string());
    }

    // 3. Cookies: check environment variable, data_dir, current directory, or user config
    if let Some(cookie_path) = find_cookies_file(data_dir) {
        args.push("--cookies".to_string());
        args.push(cookie_path.to_string_lossy().to_string());
    }

    // 4. Proxy: check YOINK_PROXY, ALL_PROXY, HTTPS_PROXY, HTTP_PROXY
    if let Some(proxy) = find_proxy() {
        args.push("--proxy".to_string());
        args.push(proxy);
    }

    args
}

fn detect_js_runtime() -> Option<&'static str> {
    if is_command_available("node") {
        Some("node")
    } else if is_command_available("deno") {
        Some("deno")
    } else if is_command_available("bun") {
        Some("bun")
    } else {
        None
    }
}

fn is_command_available(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn find_cookies_file(data_dir: Option<&Path>) -> Option<PathBuf> {
    if let Ok(path_str) = std::env::var("YOINK_COOKIES_PATH") {
        let p = PathBuf::from(path_str);
        if p.exists() {
            return Some(p);
        }
    }

    if let Some(dir) = data_dir {
        let p = dir.join("cookies.txt");
        if p.exists() {
            return Some(p);
        }
    }

    let local_p = PathBuf::from("cookies.txt");
    if local_p.exists() {
        return Some(local_p);
    }

    if let Some(user_dirs) = directories::ProjectDirs::from("", "", "yoink") {
        let conf_p = user_dirs.config_dir().join("cookies.txt");
        if conf_p.exists() {
            return Some(conf_p);
        }
    }

    None
}

fn find_proxy() -> Option<String> {
    std::env::var("YOINK_PROXY")
        .or_else(|_| std::env::var("ALL_PROXY"))
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .ok()
        .filter(|s| !s.trim().is_empty())
}
