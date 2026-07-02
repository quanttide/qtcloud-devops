//! 编排层测试：用 PATH mock 替代外部命令，验证 I/O 编排行为。
//!
//! 原理：将 mock 脚本写入临时 bin/ 目录，前置到 PATH，
//! 然后调用真实的库函数。`Command::new("gh")` 会找到我们的 mock 而非真实命令。
//!
//! 注意：这些测试会修改全局 PATH，必须串行执行。

use std::path::{Path, PathBuf};
use std::process::Command;

// ═══════════════════════════════════════════════════════════════════════
// Mock 工具
// ═══════════════════════════════════════════════════════════════════════

/// 创建返回固定输出的 shell 脚本内容。
fn mock_script(stdout: &str, stderr: &str, exit_code: i32) -> String {
    format!(
        "#!/bin/sh\ncat <<'ENDMOCK'\n{stdout}\nENDMOCK\ncat <<'ENDMOCK' >&2\n{stderr}\nENDMOCK\nexit {exit_code}\n",
    )
}

/// 创建一个模拟"命令未安装"的脚本。
fn mock_not_found() -> String {
    "#!/bin/sh\nexit 127\n".into()
}

/// 创建一个自定义 mock 脚本。
fn mock_custom(body: &str) -> String {
    format!("#!/bin/sh\n{body}\n")
}

/// 在 mock 环境中运行闭包。
fn with_mock_env<F: FnOnce() -> R, R>(scripts: &[(&str, &str)], f: F) -> R {
    let dir = tempfile::tempdir().expect("创建 temp dir");
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).expect("创建 bin/");
    for (name, body) in scripts {
        let path = bin.join(name);
        std::fs::write(&path, body).unwrap_or_else(|e| panic!("写入 mock {name}: {e}"));
        #[cfg(unix)]
        Command::new("chmod")
            .args(["+x", path.to_str().unwrap()])
            .output()
            .expect("chmod +x");
    }
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{}", bin.display(), old_path));
    let result = f();
    std::env::set_var("PATH", &old_path);
    result
}

/// 创建 mock git 仓库（简化重复的 git init / commit）。
fn git_init(path: &Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "mock@test"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Mock"])
        .current_dir(path)
        .output()
        .unwrap();
    std::fs::write(path.join(".gitkeep"), "").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();
}

/// 创建 mock git repo。
fn setup_repo() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().expect("temp dir");
    let path = d.path().to_path_buf();
    git_init(&path);
    (d, path)
}

/// 创建 mock git repo + 契约文件。
fn setup_repo_with_contract() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().expect("temp dir");
    let path = d.path().to_path_buf();
    git_init(&path);
    let contract_dir = path.join(".quanttide/devops");
    std::fs::create_dir_all(&contract_dir).unwrap();
    std::fs::write(
        contract_dir.join("contract.yaml"),
        "stages:\n  build:\n    command: cargo build\n  test:\n    command: cargo test\n    threshold: 80\n  release:\n    changelog: CHANGELOG.md\nplatform:\n  source_control: Github\n  pipeline: GithubActions\n  artifact_registry: Crates\nsource:\n  version:\n    source_type: Cargo\n    path: Cargo.toml\nscopes:\n  - name: cli\n    dir: .\n    language: Rust\n    framework: \"\"\n    build_tool: Cargo\n    registry: Crates\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add contract"])
        .current_dir(&path)
        .output()
        .unwrap();
    (d, path)
}

// ═══════════════════════════════════════════════════════════════════════
// build status 场景
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_build_status_gh_not_found() {
    let (_d, path) = setup_repo_with_contract();
    with_mock_env(&[("gh", &mock_not_found())], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_gh_empty_array() {
    let (_d, path) = setup_repo_with_contract();
    with_mock_env(&[("gh", &mock_script("[]", "", 0))], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_gh_success_run() {
    let (_d, path) = setup_repo_with_contract();
    let gh_out =
        r#"[{"conclusion":"success","displayTitle":"CI","headBranch":"main","number":42}]"#;
    with_mock_env(&[("gh", &mock_script(gh_out, "", 0))], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_gh_failed_run() {
    let (_d, path) = setup_repo_with_contract();
    let gh_out =
        r#"[{"conclusion":"failure","displayTitle":"Build","headBranch":"feat/x","number":7}]"#;
    with_mock_env(&[("gh", &mock_script(gh_out, "", 0))], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_gh_cancelled_run() {
    let (_d, path) = setup_repo_with_contract();
    let gh_out =
        r#"[{"conclusion":"cancelled","displayTitle":"CI","headBranch":"main","number":99}]"#;
    with_mock_env(&[("gh", &mock_script(gh_out, "", 0))], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_gh_unknown_conclusion() {
    let (_d, path) = setup_repo_with_contract();
    let gh_out =
        r#"[{"conclusion":"neutral","displayTitle":"Check","headBranch":"main","number":1}]"#;
    with_mock_env(&[("gh", &mock_script(gh_out, "", 0))], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_cargo_check_success() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let cargo_ok = mock_custom("exit 0");
    with_mock_env(&[("cargo", &cargo_ok)], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_cargo_check_failure() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let cargo_fail = mock_custom("exit 1");
    with_mock_env(&[("cargo", &cargo_fail)], || {
        qtcloud_devops_cli::build::status(&path);
    });
}

#[test]
fn test_build_status_no_manifest_skips_cargo() {
    let (_d, path) = setup_repo();
    qtcloud_devops_cli::build::status(&path);
}

// ═══════════════════════════════════════════════════════════════════════
// test status 场景
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_test_status_no_contract_empty_dir() {
    let (_d, path) = setup_repo();
    let c = qtcloud_devops_cli::contract::load(&path);
    qtcloud_devops_cli::test::status(&path, &c);
}

#[test]
fn test_test_status_cargo_test_success() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let cargo_out = mock_script("test result: ok. 20 passed; 0 failed; 0 ignored", "", 0);
    let c = qtcloud_devops_cli::contract::load(&path);
    with_mock_env(&[("cargo", &cargo_out)], || {
        qtcloud_devops_cli::test::status(&path, &c);
    });
}

#[test]
fn test_test_status_cargo_test_failed() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let cargo_out = mock_script("test result: FAILED. 8 passed; 3 failed; 1 ignored", "", 0);
    let c = qtcloud_devops_cli::contract::load(&path);
    with_mock_env(&[("cargo", &cargo_out)], || {
        qtcloud_devops_cli::test::status(&path, &c);
    });
}

// ═══════════════════════════════════════════════════════════════════════
// release status 场景
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_release_status_with_tags() {
    let (_d, path) = setup_repo_with_contract();
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["tag", "cli/v0.2.0"])
        .current_dir(&path)
        .output()
        .unwrap();
    std::fs::write(
        path.join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [0.2.0]\n\ncontent\n",
    )
    .unwrap();
    qtcloud_devops_cli::release::status(&path);
}

#[test]
fn test_release_status_no_tags() {
    let (_d, path) = setup_repo_with_contract();
    qtcloud_devops_cli::release::status(&path);
}

#[test]
fn test_release_status_non_git_dir() {
    let d = tempfile::tempdir().unwrap();
    qtcloud_devops_cli::release::status(d.path());
}
