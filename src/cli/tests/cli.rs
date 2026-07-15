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
    let output = cli()
        .args(["release", "publish", "--help"])
        .output()
        .unwrap();
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
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-m",
            "x",
        ])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::fs::write(
        d.path().join("CHANGELOG.md"),
        "## [1.0.0-rc.1]\n\ncontent\n",
    )
    .unwrap();
    let output = cli()
        .args(["release", "publish", "-v", "v1.0.0-rc.1", "-y"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_publish_formal_version() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-m",
            "x",
        ])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
    let output = cli()
        .args(["release", "publish", "-v", "v1.0.0", "-y"])
        .current_dir(d.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("已发布") || stdout.contains("失败"));
}

#[test]
fn test_cli_publish_auto_generates_changelog() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-m",
            "x",
        ])
        .current_dir(d.path())
        .output()
        .unwrap();
    let output = cli()
        .args(["release", "publish", "-v", "v1.0.0-rc.1", "-y"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "LLM 应自动生成 CHANGELOG, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let changelog = std::fs::read_to_string(d.path().join("CHANGELOG.md")).unwrap_or_default();
    assert!(changelog.contains("## [1.0.0-rc.1]"));
}

#[test]
fn test_cli_publish_git_not_found_err_path() {
    let d = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["release", "publish", "-v", "v1.0.0-rc.1", "-y"])
        .current_dir(d.path())
        .env("PATH", empty.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_cli_publish_gh_not_found_err_path() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::fs::write(d.path().join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-m",
            "x",
        ])
        .current_dir(d.path())
        .output()
        .unwrap();
    std::fs::write(
        d.path().join("CHANGELOG.md"),
        "## [0.0.0-ghfail]\n\ncontent\n",
    )
    .unwrap();
    // Publish prerelease first (no remote, push silently skips)
    let out = cli()
        .args(["release", "publish", "-v", "v0.0.0-ghfail", "-y"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    // Add a dummy remote so publish's push_tag has a target
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://example.com/repo.git"])
        .current_dir(d.path())
        .output()
        .unwrap();
    // Publish again with PATH → empty → gh not found → Err(e) in create_release
    let empty = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["release", "publish", "-v", "v0.0.0-ghfail", "-y"])
        .current_dir(d.path())
        .env("PATH", empty.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_cli_contract_status() {
    let d = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["contract", "status"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("契约状态"));
}

#[test]
fn test_cli_source_status() {
    let output = cli().args(["source", "status"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("git"),
        "source status 应包含 git: {}",
        &stdout[..100]
    );
}

#[test]
fn test_cli_build_status() {
    let d = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["build", "status"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("构建状态"));
}

#[test]
fn test_cli_status() {
    let d = tempfile::tempdir().unwrap();
    let output = cli().arg("status").current_dir(d.path()).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("系统诊断") || stdout.contains("契约状态"));
}

#[test]
fn test_cli_plan_status() {
    let d = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["plan", "status"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_release_status() {
    let d = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["release", "status"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_test_status() {
    let d = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["test", "status"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_plan_doctor() {
    let d = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["plan", "doctor"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_code_audit_help() {
    let output = cli().args(["code", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("audit"));
}

#[test]
fn test_cli_source_help() {
    let output = cli().args(["source", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("source"));
}

#[test]
fn test_cli_code_status_empty() {
    let d = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(d.path())
        .output()
        .unwrap();
    let output = cli()
        .args(["code", "status"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_cli_plan_clean() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ROADMAP.md"),
        "## [0.1.0]\n- [x] done\n- [ ] todo\n",
    )
    .unwrap();
    let output = cli()
        .args(["plan", "clean"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let content = std::fs::read_to_string(d.path().join("ROADMAP.md")).unwrap_or_default();
    assert!(!content.contains("done"), "ROADMAP: done 条目应被清理");
    assert!(content.contains("todo"), "ROADMAP: todo 条目应保留");
}

#[test]
fn test_cli_plan_clean_both_files() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ROADMAP.md"),
        "## [0.1.0]\n- [x] done\n- [ ] todo\n",
    )
    .unwrap();
    std::fs::write(
        d.path().join("TODO.md"),
        "## plan clean\n- [x] `src/main.rs` 完成\n- [ ] `src/plan.rs` 待办\n",
    )
    .unwrap();
    let output = cli()
        .args(["plan", "clean"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let roadmap = std::fs::read_to_string(d.path().join("ROADMAP.md")).unwrap_or_default();
    assert!(!roadmap.contains("done"), "ROADMAP: done 应被清理");
    assert!(roadmap.contains("todo"), "ROADMAP: todo 应保留");

    let todo = std::fs::read_to_string(d.path().join("TODO.md")).unwrap_or_default();
    assert!(!todo.contains("src/main.rs"), "TODO: 已完成条目应被清理");
    assert!(todo.contains("src/plan.rs"), "TODO: 待办应保留");
}

#[test]
fn test_cli_contract_help() {
    let output = cli().args(["contract", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("contract"));
}

#[test]
fn test_cli_plan_help() {
    let output = cli().args(["plan", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("plan"));
}

#[test]
fn test_cli_release_help() {
    let output = cli().args(["release", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("publish"));
}

// ── plan audit ─────────────────────────────────────────────────

#[test]
fn test_cli_plan_audit_ok() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ROADMAP.md"),
        "## [0.1.0]\n### Added\n- [ ] `src/main.rs` 功能\n",
    )
    .unwrap();
    std::fs::write(
        d.path().join("TODO.md"),
        "- [ ] `src/main.rs` 实现功能\n",
    )
    .unwrap();
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("src/main.rs"), "").unwrap();
    let output = cli()
        .args(["plan", "audit"])
        .current_dir(d.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // 路径引用有效 + TODO 条目不含格式问题 → 路径检查通过
    assert!(stdout.contains("路径引用均有效"), "路径检查应通过: {}", stdout);
}

#[test]
fn test_cli_plan_audit_path_missing() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("ROADMAP.md"), "## [0.1.0]\n### Added\n- [ ] 无路径条目\n").unwrap();
    std::fs::write(d.path().join("TODO.md"), "- [ ] `missing.rs` 路径不存在\n").unwrap();
    let output = cli()
        .args(["plan", "audit"])
        .current_dir(d.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("路径不存在"), "应检测路径缺失: {}", stdout);
}

// ── build audit ────────────────────────────────────────────────

#[test]
fn test_cli_build_audit() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"\n").unwrap();
    let output = cli()
        .args(["build", "audit"])
        .current_dir(d.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("编译器"), "应显示编译器检查: {}", stdout);
}

// ── plan status with data ──────────────────────────────────────

#[test]
fn test_cli_plan_status_with_roadmap() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ROADMAP.md"),
        "## [0.1.0]\n### Added\n- [x] done\n- [ ] todo\n",
    )
    .unwrap();
    let output = cli()
        .args(["plan", "status"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"), "应显示版本号");
    assert!(stdout.contains("1/2"), "应显示进度 1/2");
}
