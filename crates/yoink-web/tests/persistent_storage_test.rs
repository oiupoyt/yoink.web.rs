use std::net::IpAddr;
use std::sync::Arc;
use yoink_web::config::Config;
use yoink_web::jobs::{JobManager, JobStatus};

#[tokio::test]
async fn test_persistent_library_and_cache_hit() {
    let mut cfg = Config::from_env();
    let temp_dir = tempfile::tempdir().expect("failed to create tempdir");
    cfg.data_dir = temp_dir.path().to_path_buf();
    let manager = JobManager::new(Arc::new(cfg.clone()));

    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let url = "https://youtu.be/test-cache".to_string();

    // Verify list_files method exists
    let files = manager.list_files().await;
    assert_eq!(files.len(), 0);

    // Manually complete a cached job
    let job_id = manager
        .create_job(ip, url.clone(), "best".to_string(), false)
        .await
        .expect("failed to create job");

    // Simulate completion
    let fake_file = temp_dir.path().join(&job_id).join("test.mp4");
    tokio::fs::create_dir_all(fake_file.parent().unwrap()).await.unwrap();
    tokio::fs::write(&fake_file, b"video content").await.unwrap();

    manager
        .mark_completed(&job_id, fake_file.clone(), "test.mp4".to_string(), 13)
        .await;

    // Verify it is listed in available files
    let files_after = manager.list_files().await;
    assert_eq!(files_after.len(), 1);
    assert_eq!(files_after[0].id, job_id);
    assert_eq!(files_after[0].filename.as_deref(), Some("test.mp4"));

    // Now test cache hit: requesting the same URL + format should return the cached job immediately
    let cache_hit_job_id = manager
        .create_job(ip, url.clone(), "best".to_string(), false)
        .await
        .expect("failed to request cached job");

    let cache_job = manager.get_job(&cache_hit_job_id).await.unwrap();
    assert_eq!(cache_job.status, JobStatus::Completed);
    assert_eq!(cache_job.filename.as_deref(), Some("test.mp4"));
}
