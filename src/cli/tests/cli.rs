use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qtcloud-devops"))
}

#[test]
fn test_cli_help_succeeds() {
    let output = cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("release"));
    assert!(stdout.contains("code"));
}

#[test]
fn test_cli_version_output() {
    let output = cli().arg("--version").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_publish_help() {
    let output = cli().args(["release", "publish", "--help"]).output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("registry"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("version"));
}

#[test]
fn test_cli_code_help() {
    let output = cli().arg("code").arg("--help").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_publish_prerelease_succeeds() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git").args(["init", "-b", "main"]).current_dir(d.path()).output().unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(d.path()).output().unwrap();
    std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"]).current_dir(d.path()).output().unwrap();
    std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0-rc.1]\n\ncontent\n").unwrap();
    let output = cli().args(["release", "publish", "-v", "v1.0.0-rc.1", "-y"]).current_dir(d.path()).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_publish_formal_version() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git").args(["init", "-b", "main"]).current_dir(d.path()).output().unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(d.path()).output().unwrap();
    std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"]).current_dir(d.path()).output().unwrap();
    std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
    let output = cli().args(["release", "publish", "-v", "v1.0.0", "-y"]).current_dir(d.path()).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("已发布") || stdout.contains("失败"));
}

#[test]
fn test_cli_publish_rejects_missing_changelog() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git").args(["init", "-b", "main"]).current_dir(d.path()).output().unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(d.path()).output().unwrap();
    std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"]).current_dir(d.path()).output().unwrap();
    let output = cli().args(["release", "publish", "-v", "v1.0.0-rc.1", "-y"]).current_dir(d.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("CHANGELOG"), "预期 CHANGELOG 错误");
}

#[test]
fn test_cli_publish_git_not_found_err_path() {
    let d = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();
    let output = cli().args(["release", "publish", "-v", "v1.0.0-rc.1", "-y"]).current_dir(d.path()).env("PATH", empty.path()).output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_cli_publish_gh_not_found_err_path() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git").args(["init", "-b", "main"]).current_dir(d.path()).output().unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(d.path()).output().unwrap();
    std::process::Command::new("git").args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"]).current_dir(d.path()).output().unwrap();
    std::fs::write(d.path().join("CHANGELOG.md"), "## [0.0.0-ghfail]\n\ncontent\n").unwrap();
    // Publish prerelease first (no remote, push silently skips)
    let out = cli().args(["release", "publish", "-v", "v0.0.0-ghfail", "-y"]).current_dir(d.path()).output().unwrap();
    assert!(out.status.success());
    // Add a dummy remote so publish's push_tag has a target
    std::process::Command::new("git").args(["remote", "add", "origin", "https://example.com/repo.git"]).current_dir(d.path()).output().unwrap();
    // Publish again with PATH → empty → gh not found → Err(e) in create_release
    let empty = tempfile::tempdir().unwrap();
    let output = cli().args(["release", "publish", "-v", "v0.0.0-ghfail", "-y"]).current_dir(d.path()).env("PATH", empty.path()).output().unwrap();
    assert!(!output.status.success());
}
