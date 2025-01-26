#![expect(
    clippy::tests_outside_test_module,
    reason = "Clippy does not recognise integration tests as a test module."
)]
use assert_cmd::Command;
use assert_fs::TempDir;

#[test]
#[expect(clippy::unwrap_used, reason = "Test should panic on failure.")]
fn test_init_command_creates_config() {
    let tmp_dir = TempDir::new().unwrap();
    let cfg_path = tmp_dir.join("llmcli").join("config.toml");

    std::env::set_var("XDG_CONFIG_HOME", tmp_dir.path());

    let mut cmd = Command::cargo_bin("llmcli").unwrap();

    cmd.arg("init")
        .assert()
        .success()
        .stdout(format!("Configuration initialized at: {cfg_path:?}\n"));

    assert!(cfg_path.exists());
}
