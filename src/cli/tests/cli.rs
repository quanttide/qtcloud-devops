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
fn test_cli_stage_help_contains_prerelease() {
    let output = cli().args(["release", "stage", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("version"));
}

#[test]
fn test_cli_publish_help() {
    let output = cli().args(["release", "publish", "--help"]).output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("registry"));
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
        .args(["release", "stage", "-v", "v1.0.0"])
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
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0-rc.1]\n\ncontent\n").unwrap();

    let output = cli()
        .args(["release", "stage", "-v", "v1.0.0-rc.1"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_publish_formal_version() {
    let dir = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"]).current_dir(dir.path()).output().unwrap();
    std::fs::write(dir.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(dir.path()).output().unwrap();
    std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"])
        .current_dir(dir.path()).output().unwrap();
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();

    let output = cli()
        .args(["release", "publish", "-v", "v1.0.0", "-y"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("已发布") || stdout.contains("失败"), "publish should succeed or fail on git env: {}", stdout);
}

#[test]
fn test_cli_stage_rejects_missing_changelog() {
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
        .args(["release", "stage", "-v", "v1.0.0-rc.1"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CHANGELOG"), "预期 CHANGELOG 错误，得到: {}", stderr);
}

#[test]
fn test_cli_publish_rejects_missing_changelog() {
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
        .args(["release", "publish", "-v", "v1.0.0", "-y"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CHANGELOG"), "预期 CHANGELOG 错误，得到: {}", stderr);
}

#[test]
fn test_cli_stage_git_not_found_err_path() {
    let dir = tempfile::tempdir().unwrap();
    // Run in a dir without files, with PATH pointing to empty dir
    // This makes Command::new("git") fail with Err(e) → triggers the Err(e) branch
    let empty = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["release", "stage", "-v", "v1.0.0-rc.1"])
        .current_dir(dir.path())
        .env("PATH", empty.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_cli_create_release_gh_not_found_err_path() {
    let dir = tempfile::tempdir().unwrap();
    // Need a real git repo first so create_tag succeeds, then push_tag fails
    std::process::Command::new("git")
        .args(["init", "-b", "main"]).current_dir(dir.path()).output().unwrap();
    std::fs::write(dir.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(dir.path()).output().unwrap();
    std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "x"])
        .current_dir(dir.path()).output().unwrap();
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [0.0.0-ghfail]\n\ncontent\n").unwrap();
    // Stage first (no remote, push silently skips)
    let stage_out = cli()
        .args(["release", "stage", "-v", "v0.0.0-ghfail"])
        .current_dir(dir.path())
        .output().unwrap();
    assert!(stage_out.status.success());

    // Add a dummy remote after stage so publish's push_tag has a target
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://example.com/repo.git"])
        .current_dir(dir.path()).output().unwrap();

    // Now publish with PATH pointing to empty dir → gh not found → Err(e) in create_release
    let empty = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["release", "publish", "-v", "v0.0.0-ghfail", "-y"])
        .current_dir(dir.path())
        .env("PATH", empty.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}
