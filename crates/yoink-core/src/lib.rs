pub mod args_helper;
pub mod engine;
pub mod error;
pub mod ffmpeg;
pub mod format;
pub mod manager;
pub mod mock;
pub mod platform;
pub mod probe;
pub mod progress;
pub mod traits;

pub use crate::args_helper::build_common_ytdlp_args;
pub use crate::engine::YtDlpEngine;
pub use crate::error::YoinkError;
pub use crate::ffmpeg::find_ffmpeg;
pub use crate::format::{
    build_choices, format_bytes, format_duration, format_eta, format_speed, shorten_path, truncate, wrap_text,
    DownloadChoice, RawFormat, VideoInfo,
};
pub use crate::manager::YtDlpManager;
pub use crate::mock::MockDownloader;
pub use crate::platform::{detect_platform, is_probably_url, Platform};
pub use crate::probe::{clean_ytdlp_error, run_probe};
pub use crate::progress::{parse_progress_line, ProgressEvent, PROGRESS_PREFIX, PROGRESS_TEMPLATE};
pub use crate::traits::{DownloadRequest, Downloader, ProbeResult};
