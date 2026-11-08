use yoink_core::progress::{parse_progress_line, ProgressEvent};

#[test]
fn test_parse_progress_line() {
    let line = "YOINK|1048576|10485760|None|2097152|4.5";
    let event = parse_progress_line(line, 0, 1).expect("failed to parse progress line");

    match event {
        ProgressEvent::Downloading {
            downloaded_bytes,
            total_bytes,
            speed_bps,
            eta_seconds,
            part,
            total_parts,
        } => {
            assert_eq!(downloaded_bytes, 1048576);
            assert_eq!(total_bytes, Some(10485760));
            assert_eq!(speed_bps, Some(2097152));
            assert_eq!(eta_seconds, Some(4));
            assert_eq!(part, 0);
            assert_eq!(total_parts, 1);
        }
        _ => panic!("Expected ProgressEvent::Downloading"),
    }
}
