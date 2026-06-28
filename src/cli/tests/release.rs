use std::io::Write;

fn git_init(path: &std::path::Path) {
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "t@t"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "t"])
        .current_dir(path)
        .output()
        .unwrap();
    std::fs::write(path.join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn git_commit(repo: &std::path::Path) {
    let mut f = std::fs::File::create(repo.join("f")).unwrap();
    writeln!(f, "x").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "x"])
        .current_dir(repo)
        .output()
        .unwrap();
}

#[test]
fn test_release_create_tag_uses_repo_path() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    git_commit(dir.path());
    let tag = "v999.999.999-test-repo-path";
    assert!(qtcloud_devops_cli::release::create_tag(tag, dir.path()));
    let output = std::process::Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "tag", "-l"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains(tag));
    let cwd_tags = std::process::Command::new("git")
        .args(["tag", "-l"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&cwd_tags.stdout).contains(tag));
}

#[test]
fn test_release_publish_rejects_invalid_version() {
    assert!(qtcloud_devops_cli::release::publish(
        "bad",
        tempfile::tempdir().unwrap().path(),
        true,
        None
    )
    .is_err());
}

#[test]
fn test_release_publish_auto_generates_changelog() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    let result = qtcloud_devops_cli::release::publish("v1.0.0", dir.path(), true, None);
    assert!(
        result.is_ok(),
        "publish with auto-generated CHANGELOG 应成功, 得到: {:?}",
        result
    );
    let changelog = std::fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap_or_default();
    assert!(
        changelog.contains("## [1.0.0]"),
        "CHANGELOG 应包含版本条目, 得到: {}",
        changelog
    );
}

#[test]
fn test_release_publish_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(
        dir.path().join("CHANGELOG.md"),
        "## [1.0.0-rc.1]\n\ncontent\n",
    )
    .unwrap();
    assert!(qtcloud_devops_cli::release::publish("v1.0.0-rc.1", dir.path(), true, None).is_ok());
    assert!(qtcloud_devops_cli::release::publish("v1.0.0-rc.1", dir.path(), true, None).is_ok());
}

#[test]
fn test_release_publish_without_changelog_entry() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
    let result = qtcloud_devops_cli::release::publish("v2.0.0", dir.path(), true, None);
    assert!(
        result.is_ok(),
        "LLM 应自动生成缺失的 CHANGELOG 条目, 得到: {:?}",
        result
    );
    let changelog = std::fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap_or_default();
    assert!(
        changelog.contains("## [2.0.0]"),
        "CHANGELOG 应包含自动生成的 v2.0.0 条目"
    );
    assert!(changelog.contains("## [1.0.0]"), "原有的 v1.0.0 条目应保留");
}

#[test]
fn test_release_publish_with_v_prefix_changelog() {
    // CHANGELOG 已含 [v0.1.0]（v 前缀），publish v0.1.0 不应产生重复
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(
        dir.path().join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [v0.1.0]\n\n### Added\n- init\n",
    )
    .unwrap();
    let result = qtcloud_devops_cli::release::publish("v0.1.0", dir.path(), true, None);
    assert!(result.is_ok());
    let changelog = std::fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap_or_default();
    // 应只有一个 [v0.1.0] 条目，没有 [0.1.0] 重复
    assert_eq!(changelog.matches("## [").count(), 1, "不应产生重复标题");
}

#[test]
fn test_release_publish_extract_notes_filters_headers() {
    // LLM 产物混入日期头部，extract_notes 应过滤
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(
        dir.path().join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [1.0.0] - 2026-06-28\n\n\
         ## [v1.0.0] - 2023-08-31\n\n\
         ### Added\n- init\n",
    )
    .unwrap();
    let notes =
        qtcloud_devops_cli::release::extract_notes("v1.0.0", &dir.path().join("CHANGELOG.md"))
            .unwrap_or_default();
    assert!(!notes.contains("## ["), "Release notes 应过滤 ## [ 行");
    assert!(notes.contains("### Added"));
    assert!(notes.contains("- init"));
}
