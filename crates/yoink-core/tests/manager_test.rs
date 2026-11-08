use yoink_core::YtDlpManager;

#[tokio::test]
async fn test_yt_dlp_manager_finds_binary() {
    let manager = YtDlpManager::new().expect("Failed to create manager");
    let result = manager.ensure_binary().await;
    assert!(result.is_ok(), "Should have resolved binary: {:?}", result.err());
    let path_str = result.unwrap().to_string_lossy().to_string();
    assert!(path_str.ends_with("yt-dlp") || path_str.ends_with("yt-dlp.exe"));
}
