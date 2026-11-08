use std::path::PathBuf;
use yoink_core::format::DownloadChoice;
use yoink_core::platform::is_probably_url;
use yoink_core::progress::ProgressEvent;
use yoink_core::traits::ProbeResult;

use crate::theme::{Theme, ThemeMode};

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Input {
        warning: Option<String>,
    },
    Probing {
        status: String,
    },
    Picking {
        selected_index: usize,
    },
    Downloading {
        choice: DownloadChoice,
        progress: Option<ProgressEvent>,
        processing: bool,
    },
    Done {
        filepath: PathBuf,
    },
    Error {
        message: String,
    },
}

pub struct AppState {
    pub phase: Phase,
    pub url_input: String,
    pub cursor_pos: usize,
    pub clipboard_url: Option<String>,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub probe_result: Option<ProbeResult>,
    pub theme: Theme,
    pub outcome_path: Option<PathBuf>,
    pub should_exit: bool,
}

impl AppState {
    pub fn new(initial_url: Option<String>) -> Self {
        let (phase, url_input, cursor_pos) = if let Some(url) = initial_url {
            let len = url.len();
            (Phase::Probing { status: "warming up…".into() }, url, len)
        } else {
            (Phase::Input { warning: None }, String::new(), 0)
        };

        Self {
            phase,
            url_input,
            cursor_pos,
            clipboard_url: None,
            history: Vec::new(),
            history_idx: None,
            probe_result: None,
            theme: Theme::new(ThemeMode::Auto),
            outcome_path: None,
            should_exit: false,
        }
    }

    pub fn set_url_input(&mut self, val: String) {
        self.cursor_pos = val.len();
        self.url_input = val;
    }

    pub fn submit_input(&mut self) -> Option<String> {
        let trimmed = self.url_input.trim().to_string();
        if !is_probably_url(&trimmed) {
            self.phase = Phase::Input {
                warning: Some("that doesn’t look like a link — paste a full url".into()),
            };
            None
        } else {
            self.phase = Phase::Probing {
                status: "warming up…".into(),
            };
            Some(trimmed)
        }
    }

    pub fn reset_to_input(&mut self) {
        self.phase = Phase::Input { warning: None };
        self.probe_result = None;
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.cycle();
    }
}
