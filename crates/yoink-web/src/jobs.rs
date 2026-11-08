use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{broadcast, RwLock, Semaphore};

use yoink_core::engine::YtDlpEngine;
use yoink_core::progress::ProgressEvent;
use yoink_core::traits::{DownloadRequest, Downloader};

use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Probing,
    Downloading,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobView {
    pub id: String,
    pub url: String,
    pub format: String,
    pub status: JobStatus,
    pub filename: Option<String>,
    pub filesize: Option<u64>,
    pub error: Option<String>,
    pub progress: Option<ProgressEvent>,
}

pub struct Job {
    pub id: String,
    pub url: String,
    pub format: String,
    pub status: JobStatus,
    pub filename: Option<String>,
    pub filepath: Option<PathBuf>,
    pub filesize: Option<u64>,
    pub error: Option<String>,
    pub progress: Option<ProgressEvent>,
    pub created_at: Instant,
    pub event_tx: broadcast::Sender<ProgressEvent>,
    pub abort_handle: Option<tokio::task::JoinHandle<()>>,
}

pub struct JobManager {
    config: Arc<Config>,
    jobs: Arc<RwLock<HashMap<String, Arc<RwLock<Job>>>>>,
    semaphore: Arc<Semaphore>,
    daily_usage: Arc<RwLock<HashMap<IpAddr, (u32, u64)>>>, // (count, day_epoch)
}

impl JobManager {
    pub fn new(config: Arc<Config>) -> Self {
        let max_concurrent = config.max_concurrent;
        Self {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            daily_usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let now_day = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 86400;

        let mut usage = self.daily_usage.write().await;
        let entry = usage.entry(ip).or_insert((0, now_day));
        if entry.1 != now_day {
            *entry = (0, now_day);
        }

        if entry.0 >= self.config.usage_limit_per_day {
            return false;
        }

        entry.0 += 1;
        true
    }

    pub async fn create_job(
        &self,
        _ip: IpAddr,
        url: String,
        format: String,
        _is_playlist: bool,
    ) -> Result<String, String> {
        let jobs_read = self.jobs.read().await;
        if jobs_read.len() >= self.config.max_queue {
            return Err("Queue is full, try again later".to_string());
        }
        drop(jobs_read);

        let id = uuid::Uuid::new_v4().to_string();
        let (event_tx, _) = broadcast::channel(64);

        let job = Arc::new(RwLock::new(Job {
            id: id.clone(),
            url: url.clone(),
            format: format.clone(),
            status: JobStatus::Queued,
            filename: None,
            filepath: None,
            filesize: None,
            error: None,
            progress: None,
            created_at: Instant::now(),
            event_tx: event_tx.clone(),
            abort_handle: None,
        }));

        self.jobs.write().await.insert(id.clone(), job.clone());

        // Spawn async background worker
        let sem = self.semaphore.clone();
        let config = self.config.clone();
        let job_clone = job.clone();
        let job_id = id.clone();

        let handle = tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };

            // Check if cancelled while waiting
            {
                let j = job_clone.read().await;
                if j.status == JobStatus::Cancelled {
                    return;
                }
            }

            // Update to Probing
            {
                let mut j = job_clone.write().await;
                j.status = JobStatus::Probing;
                let _ = j.event_tx.send(ProgressEvent::Probing {
                    status: "probing video info…".into(),
                });
            }

            let engine = match YtDlpEngine::new().await {
                Ok(e) => e,
                Err(e) => {
                    let mut j = job_clone.write().await;
                    j.status = JobStatus::Failed;
                    j.error = Some(e.to_string());
                    let _ = j.event_tx.send(ProgressEvent::Error {
                        message: e.to_string(),
                    });
                    return;
                }
            };

            let probe_res = match engine.probe(&url).await {
                Ok(p) => p,
                Err(e) => {
                    let mut j = job_clone.write().await;
                    j.status = JobStatus::Failed;
                    j.error = Some(e.to_string());
                    let _ = j.event_tx.send(ProgressEvent::Error {
                        message: e.to_string(),
                    });
                    return;
                }
            };

            // Pick format choice
            let choice = if format == "audio" {
                probe_res
                    .choices
                    .iter()
                    .find(|c| c.kind == "audio")
                    .cloned()
                    .unwrap_or_else(|| probe_res.choices[0].clone())
            } else if format == "1080p" || format == "720p" || format == "480p" {
                probe_res
                    .choices
                    .iter()
                    .find(|c| c.label.contains(&format))
                    .cloned()
                    .unwrap_or_else(|| probe_res.choices[0].clone())
            } else {
                probe_res.choices[0].clone()
            };

            // Update to Downloading
            {
                let mut j = job_clone.write().await;
                j.status = JobStatus::Downloading;
            }

            let job_dir = config.data_dir.join(&job_id);
            let _ = tokio::fs::create_dir_all(&job_dir).await;

            let req = DownloadRequest {
                url,
                choice,
                out_dir: job_dir,
                info_json_path: probe_res.info_json_path,
            };

            match engine.download(req).await {
                Ok((mut rx, download_handle)) => {
                    let job_inner = job_clone.clone();
                    tokio::spawn(async move {
                        while let Ok(event) = rx.recv().await {
                            let mut j = job_inner.write().await;
                            j.progress = Some(event.clone());
                            let _ = j.event_tx.send(event);
                        }
                    });

                    match download_handle.await {
                        Ok(Ok(dest)) => {
                            let size = tokio::fs::metadata(&dest).await.map(|m| m.len()).unwrap_or(0);
                            let filename = dest
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| "download.mp4".into());

                            let mut j = job_clone.write().await;
                            j.status = JobStatus::Completed;
                            j.filepath = Some(dest.clone());
                            j.filename = Some(filename);
                            j.filesize = Some(size);
                            let _ = j.event_tx.send(ProgressEvent::Finished { filepath: dest });
                        }
                        Ok(Err(e)) => {
                            let mut j = job_clone.write().await;
                            j.status = JobStatus::Failed;
                            j.error = Some(e.to_string());
                            let _ = j.event_tx.send(ProgressEvent::Error {
                                message: e.to_string(),
                            });
                        }
                        Err(e) => {
                            let mut j = job_clone.write().await;
                            j.status = JobStatus::Failed;
                            j.error = Some(e.to_string());
                            let _ = j.event_tx.send(ProgressEvent::Error {
                                message: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let mut j = job_clone.write().await;
                    j.status = JobStatus::Failed;
                    j.error = Some(e.to_string());
                    let _ = j.event_tx.send(ProgressEvent::Error {
                        message: e.to_string(),
                    });
                }
            }
        });

        job.write().await.abort_handle = Some(handle);

        Ok(id)
    }

    pub async fn get_job(&self, id: &str) -> Option<JobView> {
        let jobs = self.jobs.read().await;
        let job_arc = jobs.get(id)?;
        let j = job_arc.read().await;
        Some(JobView {
            id: j.id.clone(),
            url: j.url.clone(),
            format: j.format.clone(),
            status: j.status.clone(),
            filename: j.filename.clone(),
            filesize: j.filesize,
            error: j.error.clone(),
            progress: j.progress.clone(),
        })
    }

    pub async fn get_events(&self, id: &str) -> Option<broadcast::Receiver<ProgressEvent>> {
        let jobs = self.jobs.read().await;
        let job_arc = jobs.get(id)?;
        let j = job_arc.read().await;
        Some(j.event_tx.subscribe())
    }

    pub async fn cancel_job(&self, id: &str) -> bool {
        let mut jobs = self.jobs.write().await;
        if let Some(job_arc) = jobs.remove(id) {
            let mut j = job_arc.write().await;
            j.status = JobStatus::Cancelled;
            if let Some(handle) = j.abort_handle.take() {
                handle.abort();
            }
            if let Some(path) = &j.filepath {
                let _ = tokio::fs::remove_file(path).await;
            }
            let job_dir = self.config.data_dir.join(id);
            let _ = tokio::fs::remove_dir_all(job_dir).await;
            true
        } else {
            false
        }
    }

    pub async fn get_job_file(&self, id: &str) -> Option<(PathBuf, String)> {
        let jobs = self.jobs.read().await;
        let job_arc = jobs.get(id)?;
        let j = job_arc.read().await;
        if j.status == JobStatus::Completed {
            if let (Some(path), Some(name)) = (&j.filepath, &j.filename) {
                return Some((path.clone(), name.clone()));
            }
        }
        None
    }

    pub async fn sweep_expired_jobs(&self) {
        let ttl = Duration::from_secs(self.config.ttl_hours * 3600);
        let mut jobs = self.jobs.write().await;
        let mut expired_ids = Vec::new();

        for (id, job_arc) in jobs.iter() {
            let j = job_arc.read().await;
            if j.created_at.elapsed() >= ttl {
                expired_ids.push(id.clone());
            }
        }

        for id in expired_ids {
            if let Some(job_arc) = jobs.remove(&id) {
                let j = job_arc.read().await;
                if let Some(path) = &j.filepath {
                    let _ = tokio::fs::remove_file(path).await;
                }
                let job_dir = self.config.data_dir.join(&id);
                let _ = tokio::fs::remove_dir_all(job_dir).await;
            }
        }
    }
}
