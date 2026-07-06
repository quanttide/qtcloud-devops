use std::io::Write;

fn git_init(path: &std::path::Path) -> std::path::PathBuf {
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
    let init = path.join("init");
    std::fs::write(&init, "").unwrap();
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
    init
}

fn git_commit(repo: &std::path::Path) {
    std::fs::write(repo.join("f"), "x").unwrap();
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

fn git_tag(repo: &std::path::Path, tag: &str) {
    std::process::Command::new("git")
        .args(["-C", repo.to_str().unwrap(), "tag", tag])
        .output()
        .unwrap();
}

// ═════════════════════════════════════════════════════════════════════
// release::status 集成测试
// ═════════════════════════════════════════════════════════════════════

#[test]
fn test_status_empty_repo_no_tags() {
    // 空仓库无标签 → status 不 panic、不修改 repo
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    qtcloud_devops_cli::release::status(dir.path());
    // read-only 操作，目录无新增文件
    assert!(!dir.path().join("CHANGELOG.md").exists());
}

#[test]
fn test_status_with_root_tag() {
    // 有根 scope tag → 打印标签信息
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    git_tag(dir.path(), "v1.0.0");
    qtcloud_devops_cli::release::status(dir.path());
    // tag 仍存在
    let output = std::process::Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "tag"])
        .output()
        .unwrap();
    let tags = String::from_utf8_lossy(&output.stdout);
    assert!(tags.contains("v1.0.0"));
}

#[test]
fn test_status_with_scoped_tags() {
    // 多 scope tag → 按 scope 分组输出
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    git_tag(dir.path(), "cli/v0.1.0");
    git_tag(dir.path(), "web/v0.2.0");
    qtcloud_devops_cli::release::status(dir.path());
}

#[test]
fn test_status_dirty_workspace() {
    // 未提交变更 → 工作区标记 dirty
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    git_tag(dir.path(), "v1.0.0");
    std::fs::write(dir.path().join("dirty"), "modified").unwrap();
    qtcloud_devops_cli::release::status(dir.path());
}

#[test]
fn test_status_with_changelog() {
    // CHANGELOG 存在且匹配 tag → CHANGELOG ✅
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
    git_commit(dir.path()); // commit CHANGELOG
    git_tag(dir.path(), "v1.0.0");
    qtcloud_devops_cli::release::status(dir.path());
}

#[test]
fn test_status_missing_changelog() {
    // 有 tag 无 CHANGELOG → CHANGELOG ❌
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    // 不在 CHANGELOG 中写版本，直接打 tag
    git_tag(dir.path(), "v1.0.0");
    qtcloud_devops_cli::release::status(dir.path());
}

#[test]
fn test_status_with_unreleased_commits() {
    // tag 后有新提交 → 未发布提交计数 > 0
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    git_tag(dir.path(), "v1.0.0");
    git_commit(dir.path());
    git_commit(dir.path());
    qtcloud_devops_cli::release::status(dir.path());
}

#[test]
fn test_status_no_remote_no_gh() {
    // 无 remote origin → 跳过 GitHub Release 检查（不 panic）
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    git_tag(dir.path(), "v1.0.0");
    qtcloud_devops_cli::release::status(dir.path());
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
        Some("bad"),
        tempfile::tempdir().unwrap().path(),
        true,
        false,
        false,
        None
    )
    .is_err());
}

#[test]
fn test_release_publish_auto_generates_changelog() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    let result =
        qtcloud_devops_cli::release::publish(Some("v1.0.0"), dir.path(), true, false, false, None);
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
    assert!(qtcloud_devops_cli::release::publish(
        Some("v1.0.0-rc.1"),
        dir.path(),
        true,
        false,
        false,
        None
    )
    .is_ok());
    assert!(qtcloud_devops_cli::release::publish(
        Some("v1.0.0-rc.1"),
        dir.path(),
        true,
        false,
        false,
        None
    )
    .is_ok());
}

#[test]
fn test_release_publish_without_changelog_entry() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
    let result =
        qtcloud_devops_cli::release::publish(Some("v2.0.0"), dir.path(), true, false, false, None);
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
    let result =
        qtcloud_devops_cli::release::publish(Some("v0.1.0"), dir.path(), true, false, false, None);
    assert!(result.is_ok());
    let changelog = std::fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap_or_default();
    // 应只有一个 [v0.1.0] 条目，没有 [0.1.0] 重复
    assert_eq!(changelog.matches("## [").count(), 1, "不应产生重复标题");
}

#[test]
fn test_release_publish_extract_notes() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(
        dir.path().join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [1.0.0] - 2026-06-28\n\n### Added\n- init\n",
    )
    .unwrap();
    let notes =
        qtcloud_devops_cli::release::extract_notes("v1.0.0", &dir.path().join("CHANGELOG.md"))
            .unwrap_or_default();
    assert!(notes.contains("### Added"), "应包含分类: {}", notes);
    assert!(notes.contains("- init"), "应包含条目: {}", notes);
}

#[test]
fn test_release_publish_scoped_monorepo() {
    // monorepo: git 根 ≠ scope 子目录
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());

    // 创建契约
    let contract_dir = dir.path().join(".quanttide/devops");
    std::fs::create_dir_all(&contract_dir).unwrap();
    std::fs::write(
        contract_dir.join("contract.yaml"),
        "scopes:\n  cli:\n    dir: packages/cli\n    language: rust\n",
    )
    .unwrap();

    // 创建 scope 子目录
    let scope_dir = dir.path().join("packages/cli");
    std::fs::create_dir_all(&scope_dir).unwrap();

    // scope 的配置文件
    std::fs::write(
        scope_dir.join("Cargo.toml"),
        "[package]\nname = \"cli\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        scope_dir.join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [0.1.0]\n\n### Added\n- init\n",
    )
    .unwrap();

    // 提交所有文件
    git_commit(dir.path());

    // 发布 scoped 版本 —— repo_path=git根, scope_dir=contract映射的子目录
    let result = qtcloud_devops_cli::release::publish(
        Some("cli/v0.2.0"),
        dir.path(),
        true,
        false,
        false,
        None,
    );
    assert!(
        result.is_ok(),
        "monorepo scoped publish 应成功: {:?}",
        result
    );

    // 验证 scope 目录文件已更新
    let cargo = std::fs::read_to_string(scope_dir.join("Cargo.toml")).unwrap_or_default();
    assert!(
        cargo.contains("version = \"0.2.0\""),
        "scope 的 Cargo.toml 应更新: {}",
        cargo
    );

    // 验证 CHANGELOG 在 scope 目录下（不在根目录）
    let scope_changelog =
        std::fs::read_to_string(scope_dir.join("CHANGELOG.md")).unwrap_or_default();
    assert!(
        scope_changelog.contains("## [0.2.0]"),
        "scope 的 CHANGELOG 应含新版本: {}",
        scope_changelog
    );
    let root_changelog = dir.path().join("CHANGELOG.md");
    assert!(!root_changelog.exists(), "根目录不应生成 CHANGELOG");

    // 验证 tag 存在
    let output = std::process::Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "tag",
            "-l",
            "cli/v0.2.0",
        ])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("cli/v0.2.0"),
        "tag 应存在"
    );
}
