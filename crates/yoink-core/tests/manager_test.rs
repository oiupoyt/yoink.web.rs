use yoink_core::YtDlpManager;

#[tokio::test]
async fn test_yt_dlp_manager_finds_system_binary() {
    let manager = YtDlpManager::new().expect("Failed to create manager");
    let result = manager.ensure_binary().await;
    // yt-dlp is in the PATH, so it should be found.
    assert!(result.is_ok(), "Should have found the binary: {:?}", result.err());
    assert_eq!(result.unwrap().to_str().unwrap(), "yt-dlp");
}
