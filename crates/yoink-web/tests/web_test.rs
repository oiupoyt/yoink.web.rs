use yoink_web::config::Config;
use yoink_web::jobs::JobManager;
use std::net::IpAddr;
use std::sync::Arc;

#[tokio::test]
async fn test_web_config_defaults() {
    let cfg = Config::from_env();
    assert_eq!(cfg.port, 8080);
    assert_eq!(cfg.max_concurrent, 3);
}

#[tokio::test]
async fn test_job_lifecycle() {
    let cfg = Config::from_env();
    let manager = JobManager::new(Arc::new(cfg));

    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let job_id = manager
        .create_job(ip, "https://youtu.be/dQw4w9WgXcQ".to_string(), "best".to_string(), false)
        .await
        .expect("failed to create job");

    let job_view = manager.get_job(&job_id).await.expect("job not found");
    assert_eq!(job_view.id, job_id);
    assert_eq!(job_view.status, yoink_web::jobs::JobStatus::Queued);
}
