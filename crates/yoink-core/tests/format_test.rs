use yoink_core::format::{build_choices, format_bytes, format_duration, RawFormat, VideoInfo};

#[test]
fn test_format_helpers() {
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(10485760), "10 MB");
    assert_eq!(format_duration(65.0), "1:05");
    assert_eq!(format_duration(3665.0), "1:01:05");
}

#[test]
fn test_build_choices() {
    let info = VideoInfo {
        title: "Test Video".to_string(),
        uploader: Some("Tester".to_string()),
        duration: Some(120.0),
        webpage_url: Some("https://youtu.be/dQw4w9WgXcQ".to_string()),
        extractor_key: Some("Youtube".to_string()),
        formats: Some(vec![
            RawFormat {
                format_id: "137".to_string(),
                ext: Some("mp4".to_string()),
                vcodec: Some("avc1".to_string()),
                acodec: Some("none".to_string()),
                height: Some(1080),
                width: Some(1920),
                abr: None,
                tbr: Some(4000.0),
                filesize: Some(50_000_000),
                filesize_approx: None,
            },
            RawFormat {
                format_id: "140".to_string(),
                ext: Some("m4a".to_string()),
                vcodec: Some("none".to_string()),
                acodec: Some("mp4a".to_string()),
                height: None,
                width: None,
                abr: Some(128.0),
                tbr: Some(128.0),
                filesize: Some(5_000_000),
                filesize_approx: None,
            },
        ]),
        is_playlist: false,
        entries: None,
    };

    let choices = build_choices(&info);
    assert!(!choices.is_empty());
    assert_eq!(choices[0].kind, "video");
    assert!(choices[0].label.starts_with("1080p · mp4"));
    // Last choice should always be audio only mp3
    let last = choices.last().unwrap();
    assert_eq!(last.kind, "audio");
    assert!(last.label.starts_with("audio only · mp3"));
}
