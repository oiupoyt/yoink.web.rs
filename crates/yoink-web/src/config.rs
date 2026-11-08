use ipnet::IpNet;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub api_key: Option<String>,
    pub ip_allowlist: Option<Vec<IpNet>>,
    pub max_concurrent: usize,
    pub max_file_mb: u64,
    pub max_queue: usize,
    pub ttl_hours: u64,
    pub usage_limit_per_day: u32,
    pub update_interval_hours: u64,
}

impl Config {
    pub fn from_env() -> Self {
        let bind = std::env::var("YOINK_BIND").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("YOINK_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        let data_dir = std::env::var("YOINK_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data"));
        let api_key = std::env::var("YOINK_API_KEY").ok().filter(|s| !s.is_empty());

        let ip_allowlist = std::env::var("YOINK_IP_ALLOWLIST").ok().and_then(|val| {
            let list: Vec<IpNet> = val
                .split(',')
                .filter_map(|s| IpNet::from_str(s.trim()).ok())
                .collect();
            if list.is_empty() {
                None
            } else {
                Some(list)
            }
        });

        let max_concurrent = std::env::var("YOINK_MAX_CONCURRENT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        let max_file_mb = std::env::var("YOINK_MAX_FILE_MB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2048);

        let max_queue = std::env::var("YOINK_MAX_QUEUE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);

        let ttl_hours = std::env::var("YOINK_TTL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);

        let usage_limit_per_day = std::env::var("YOINK_USAGE_LIMIT_PER_DAY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);

        let update_interval_hours = std::env::var("YOINK_UPDATE_INTERVAL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);

        Self {
            bind,
            port,
            data_dir,
            api_key,
            ip_allowlist,
            max_concurrent,
            max_file_mb,
            max_queue,
            ttl_hours,
            usage_limit_per_day,
            update_interval_hours,
        }
    }
}
