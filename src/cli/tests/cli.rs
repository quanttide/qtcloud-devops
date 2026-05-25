use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qtcloud-devops"))
}

#[test]
fn test_cli_help_succeeds() {
    let output = cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stage"));
    assert!(stdout.contains("publish"));
    assert!(stdout.contains("retire"));
    assert!(stdout.contains("release"));
}

#[test]
fn test_cli_version_output() {
    let output = cli().arg("--version").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_stage_help_contains_prerelease() {
    let output = cli().arg("stage").arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("version"));
}

#[test]
fn test_cli_publish_help() {
    let output = cli().arg("publish").arg("--help").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("registry"));
}

#[test]
fn test_cli_retire_help() {
    let output = cli().arg("retire").arg("--help").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_release_status_help() {
    let output = cli().arg("release").arg("status").arg("--help").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_code_help() {
    let output = cli().arg("code").arg("--help").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_stage_rejects_formal_version() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"]).current_dir(dir.path()).output().unwrap();
    std::fs::write(dir.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(dir.path()).output().unwrap();
    std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"])
        .current_dir(dir.path()).output().unwrap();

    let output = cli()
        .args(["stage", "-v", "v1.0.0"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("仅用于预发布"));
}

#[test]
fn test_cli_stage_prerelease_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"]).current_dir(dir.path()).output().unwrap();
    std::fs::write(dir.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(dir.path()).output().unwrap();
    std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"])
        .current_dir(dir.path()).output().unwrap();

    let output = cli()
        .args(["stage", "-v", "v1.0.0-rc.1"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Staged"));
}
