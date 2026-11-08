use std::path::PathBuf;
use yoink_core::traits::{Downloader, DownloadRequest};
use yoink_core::mock::MockDownloader;
use yoink_core::progress::ProgressEvent;

#[tokio::test]
async fn test_mock_downloader() {
    let mock = MockDownloader::new("Mock_Video_Title".to_string());
    let probe_res = mock.probe("https://youtu.be/dummy").await.expect("probe failed");
    assert_eq!(probe_res.info.title, "Mock_Video_Title");
    assert!(!probe_res.choices.is_empty());

    let req = DownloadRequest {
        url: "https://youtu.be/dummy".to_string(),
        choice: probe_res.choices[0].clone(),
        out_dir: PathBuf::from("/tmp/yoink-test"),
        info_json_path: None,
    };

    let (mut rx, handle) = mock.download(req).await.expect("download failed");
    let mut got_downloading = false;
    let mut got_finished = false;

    while let Ok(event) = rx.recv().await {
        match event {
            ProgressEvent::Downloading { .. } => got_downloading = true,
            ProgressEvent::Finished { .. } => got_finished = true,
            _ => {}
        }
    }

    let final_path = handle.await.expect("task join failed").expect("download execution failed");
    assert!(got_downloading);
    assert!(got_finished);
    assert!(final_path.to_str().unwrap().contains("Mock_Video_Title"));
}
