use yoink_core::engine::YtDlpEngine;
use yoink_core::traits::Downloader;

#[tokio::test]
async fn test_ytdlp_engine_probe_failure_on_invalid_url() {
    let engine = YtDlpEngine::new().await.expect("failed to init engine");
    let result = engine.probe("https://invalid-domain-that-does-not-exist-12345.com/video").await;
    assert!(result.is_err(), "Expected error on nonexistent domain");
}
