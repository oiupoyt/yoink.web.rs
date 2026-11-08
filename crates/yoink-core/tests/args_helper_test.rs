use yoink_core::args_helper::build_common_ytdlp_args;

#[test]
fn test_common_ytdlp_args_includes_tv_client_and_js_runtime() {
    let args = build_common_ytdlp_args(None);
    assert!(args.iter().any(|a| a == "--extractor-args"));
    assert!(args.iter().any(|a| a.contains("player_client=")));
    // Should pass --js-runtimes if node/bun/deno exists on system
    if std::process::Command::new("node").arg("--version").output().is_ok() {
        assert!(args.iter().any(|a| a == "--js-runtimes"));
    }
}
