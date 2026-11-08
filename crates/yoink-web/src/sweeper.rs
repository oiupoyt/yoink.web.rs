use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::jobs::JobManager;

pub fn start_sweeper(manager: Arc<JobManager>) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(600)).await; // every 10 minutes
            manager.sweep_expired_jobs().await;
        }
    });
}
