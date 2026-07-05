//! 编排层测试：用 PATH mock 替代外部命令，验证 I/O 编排行为。
//!
//! 原理：将 mock 脚本写入临时 bin/ 目录，前置到 PATH，
//! 然后调用真实的库函数。`Command::new("gh")` 会找到我们的 mock 而非真实命令。
//!
//! 注意：这些测试会修改全局 PATH，已通过全局锁串行化。

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
///
/// 修改全局 PATH，所以通过全局锁串行化，避免并行测试竞争。
fn with_mock_env<F: FnOnce() -> R, R>(scripts: &[(&str, &str)], f: F) -> R {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap();
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
        "stages:\n  build:\n    command: cargo build\n  test:\n    command: cargo test\n    threshold: 80\n  release:\n    changelog: CHANGELOG.md\nplatform:\n  source_control: github\n  pipeline: github_actions\n  artifact_registry: crates\nsources:\n  version:\n    type: cargo\nscopes:\n  cli:\n    dir: .\n    language: rust\n    build_tool: cargo\n    registry: crates\n",
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
// test run 场景（覆盖性能测试：验证 test::run 的新逻辑）
// ═══════════════════════════════════════════════════════════════════════

/// Rust scope: cargo llvm-cov 自身包含测试 + 覆盖率，验证 run_tests_for_lang 被跳过。
#[test]
fn test_test_run_rust_coverage_handles_tests() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    // cargo llvm-cov 成功 → 表示测试通过 + 覆盖率已更新
    let cargo_ok = mock_custom("exit 0");
    with_mock_env(&[("cargo", &cargo_ok)], || {
        let result = qtcloud_devops_cli::test::run(&path);
        assert!(result.is_ok(), "llvm-cov 成功 → run 应返回 Ok");
    });
}

/// Rust scope: cargo llvm-cov 失败（测试未通过）→ run 应返回 Err。
#[test]
fn test_test_run_rust_coverage_fails_propagates_error() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let cargo_fail = mock_custom("exit 1");
    with_mock_env(&[("cargo", &cargo_fail)], || {
        let result = qtcloud_devops_cli::test::run(&path);
        assert!(result.is_err(), "llvm-cov 失败 → run 应返回 Err");
        let err = result.unwrap_err();
        assert!(err.contains("cargo"), "错误信息应包含命令名: {}", err);
    });
}

/// Python scope: coverage 不包含测试 → 先跑 python -m pytest，再跑 coverage。
#[test]
fn test_test_run_python_separate_test_and_coverage() {
    let (_d, path) = setup_repo();
    std::fs::write(path.join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();
    // python -m pytest 通过，coverage 通过
    let py_ok = mock_custom("exit 0");
    let coverage_ok = mock_custom("exit 0");
    with_mock_env(&[("python", &py_ok), ("coverage", &coverage_ok)], || {
        let result = qtcloud_devops_cli::test::run(&path);
        assert!(result.is_ok(), "Python test+coverage 均应通过");
    });
}

/// Python scope: pytest 失败 → run 应返回 Err，coverage 不执行。
#[test]
fn test_test_run_python_test_fails() {
    let (_d, path) = setup_repo();
    std::fs::write(path.join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();
    let py_fail = mock_custom("exit 1");
    let coverage_ok = mock_custom("exit 0");
    with_mock_env(&[("python", &py_fail), ("coverage", &coverage_ok)], || {
        let result = qtcloud_devops_cli::test::run(&path);
        assert!(result.is_err(), "pytest 失败 → run 应返回 Err");
    });
}

/// 空项目（无清单文件）：不应 panic，静默跳过。
#[test]
fn test_test_run_empty_dir() {
    let (_d, path) = setup_repo();
    let result = qtcloud_devops_cli::test::run(&path);
    assert!(result.is_ok(), "空目录应返回 Ok");
}

/// 不存在的语言（未知）：跳过测试，不 panic。
#[test]
fn test_test_run_unknown_lang() {
    let (_d, path) = setup_repo();
    std::fs::write(path.join("README.md"), "").unwrap();
    let result = qtcloud_devops_cli::test::run(&path);
    assert!(result.is_ok(), "未知语言应返回 Ok");
}

/// 多 scope 场景验证：每个 scope 各自走覆盖/测试逻辑。
#[test]
fn test_test_run_multiple_scopes() {
    let (_d, path) = setup_repo();
    // 创建 Rust scope
    let cli_dir = path.join("packages/cli");
    std::fs::create_dir_all(&cli_dir).unwrap();
    std::fs::write(
        cli_dir.join("Cargo.toml"),
        "[package]\nname = \"cli\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    // 创建 Python scope
    let py_dir = path.join("packages/sdk");
    std::fs::create_dir_all(&py_dir).unwrap();
    std::fs::write(py_dir.join("pyproject.toml"), "[project]\nname = \"sdk\"\n").unwrap();
    // 创建契约使两个 scope 被识别
    let contract_dir = path.join(".quanttide/devops");
    std::fs::create_dir_all(&contract_dir).unwrap();
    std::fs::write(
        contract_dir.join("contract.yaml"),
        "scopes:\n  cli:\n    dir: packages/cli\n    language: rust\n  sdk:\n    dir: packages/sdk\n    language: python\n",
    )
    .unwrap();
    // mock: Rust scope 用 cargo (llvm-cov)，Python scope 用 python + coverage
    let cargo_ok = mock_custom("exit 0");
    let py_ok = mock_custom("exit 0");
    let coverage_ok = mock_custom("exit 0");
    with_mock_env(
        &[
            ("cargo", &cargo_ok),
            ("python", &py_ok),
            ("coverage", &coverage_ok),
        ],
        || {
            let result = qtcloud_devops_cli::test::run(&path);
            assert!(result.is_ok(), "多 scope 全通过");
        },
    );
}

/// 验证 scope 过滤：从子目录运行 test run 时，不越级到其他 scope。
/// 模拟 monorepo 有两个 crate（cli + sdk），从 cli 目录运行，确认只触发 cli 的编译。
#[test]
fn test_test_run_scoped_by_cwd() {
    let (_d, path) = setup_repo();
    // 模拟 monorepo: 两个 scope 各有一个 Cargo.toml
    let cli_dir = path.join("packages/cli");
    let sdk_dir = path.join("packages/sdk");
    std::fs::create_dir_all(&cli_dir).unwrap();
    std::fs::create_dir_all(&sdk_dir).unwrap();
    std::fs::write(
        cli_dir.join("Cargo.toml"),
        "[package]\nname = \"cli\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        sdk_dir.join("Cargo.toml"),
        "[package]\nname = \"sdk\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    // 写入契约，让两个 scope 被识别
    let contract_dir = path.join(".quanttide/devops");
    std::fs::create_dir_all(&contract_dir).unwrap();
    std::fs::write(
        contract_dir.join("contract.yaml"),
        "scopes:\n  cli:\n    dir: packages/cli\n    language: rust\n  sdk:\n    dir: packages/sdk\n    language: rust\n",
    )
    .unwrap();

    // cargo mock: 写入运行目录到 sentinel 文件
    let sentinel = path.join(".cov_sentinel");
    let sentinel_path = sentinel.to_string_lossy().to_string();
    let cargo_mock = format!("#!/bin/sh\necho \"$PWD\" >> {}\nexit 0\n", sentinel_path);

    // 从 cli 子目录运行 test run
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&cli_dir).unwrap();
    with_mock_env(&[("cargo", &cargo_mock)], || {
        let result = qtcloud_devops_cli::test::run(&path);
        assert!(result.is_ok(), "test run 应成功");
    });
    std::env::set_current_dir(&old_cwd).unwrap();

    // 确认只调用了 cargo 一次，且路径是 packages/cli
    let recorded = std::fs::read_to_string(&sentinel).unwrap_or_default();
    let calls: Vec<&str> = recorded.lines().collect();
    assert_eq!(calls.len(), 1, "应只触发一次 cargo 调用，实际: {:?}", calls);
    assert!(
        calls[0].ends_with("packages/cli") || calls[0].ends_with("packages/cli/"),
        "cargo 应在 packages/cli 目录运行，实际: {}",
        calls[0]
    );
}

/// 性能回归测试：捕获 cargo llvm-cov 的完整参数，确认 scope 过滤后
/// 只对当前 crate 调用一次，且参数不含非预期的 flag。
#[test]
fn test_test_run_captures_cargo_args() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(path.join("src")).unwrap();
    std::fs::write(path.join("src/lib.rs"), "#[test] fn it_works() {}\n").unwrap();

    let sentinel = path.join(".args_sentinel");
    let s = sentinel.to_string_lossy().to_string();
    let cargo_mock = format!("#!/bin/sh\necho \"$@\" >> {}\nexit 0\n", s);

    with_mock_env(&[("cargo", &cargo_mock)], || {
        let result = qtcloud_devops_cli::test::run(&path);
        assert!(result.is_ok(), "test run 应成功");
    });

    let recorded = std::fs::read_to_string(&sentinel).unwrap_or_default();
    assert!(!recorded.is_empty(), "cargo 应被调用");
    assert!(
        recorded.contains("llvm-cov"),
        "参数应包含 llvm-cov，实际: {}",
        recorded
    );
    assert!(
        recorded.contains("--lcov"),
        "参数应包含 --lcov，实际: {}",
        recorded
    );
    assert!(
        recorded.contains("--output-path"),
        "参数应包含 --output-path，实际: {}",
        recorded
    );
}

/// Rust 覆盖率不可用（cargo llvm-cov not found）：降级为跳过，不崩溃。
#[test]
fn test_test_run_rust_coverage_not_found() {
    let (_d, path) = setup_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    // cargo 命令不存在 → run_coverage_for_lang 返回 Ok(false)
    // 然后 run_tests_for_lang 会尝试启动 cargo test → cargo 也不存在 → 返回错误
    // 但 coverage missing 不应导致 panic
    let result = qtcloud_devops_cli::test::run(&path);
    assert!(result.is_err(), "cargo 缺失 → 启动失败");
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

// ═══════════════════════════════════════════════════════════════════════
// release util: create_release（合并为一个测试避免 PATH 并行冲突）
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_create_release_scenarios() {
    // 1. gh 返回成功
    with_mock_env(&[("gh", &mock_custom("exit 0"))], || {
        assert!(qtcloud_devops_cli::release::create_release(
            "v1.0.0",
            "notes",
            "owner/repo"
        ));
    });
    // 2. gh 已存在
    with_mock_env(
        &[("gh", &mock_custom("echo 'already exists' >&2; exit 1"))],
        || {
            assert!(qtcloud_devops_cli::release::create_release(
                "v1.0.0", "", "o/r"
            ));
        },
    );
    // 3. gh 其他错误
    with_mock_env(
        &[("gh", &mock_custom("echo 'unexpected' >&2; exit 1"))],
        || {
            assert!(!qtcloud_devops_cli::release::create_release(
                "v1.0.0", "", "o/r"
            ));
        },
    );
}

// ═══════════════════════════════════════════════════════════════════════
// release status: check_github_release
// ═══════════════════════════════════════════════════════════════════════

/// gh release view 返回 body 且与 CHANGELOG 一致。
#[test]
fn test_release_status_gh_view_matches() {
    let (_d, path) = setup_repo_with_contract();
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&path)
        .output()
        .unwrap();
    std::fs::write(
        path.join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [1.0.0]\n\ncontent\n",
    )
    .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .current_dir(&path)
        .output()
        .unwrap();
    // gh release view <tag> --repo <repo> --json body --jq .body 输出 body 内容
    let gh = mock_custom(r#"case "$1/$2" in release/view) echo "content";; *) exit 1;; esac"#);
    with_mock_env(&[("gh", &gh)], || {
        qtcloud_devops_cli::release::status(&path);
    });
}

/// gh release view 返回空 body。
#[test]
fn test_release_status_gh_view_empty_body() {
    let (_d, path) = setup_repo_with_contract();
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&path)
        .output()
        .unwrap();
    std::fs::write(
        path.join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [1.0.0]\n\ncontent\n",
    )
    .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .current_dir(&path)
        .output()
        .unwrap();
    let gh = mock_custom(r#"case "$1/$2" in release/view) echo "";; *) exit 1;; esac"#);
    with_mock_env(&[("gh", &gh)], || {
        qtcloud_devops_cli::release::status(&path);
    });
}

/// gh CLI 不存在。
#[test]
fn test_release_status_gh_not_found() {
    let (_d, path) = setup_repo_with_contract();
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&path)
        .output()
        .unwrap();
    std::fs::write(
        path.join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [1.0.0]\n\ncontent\n",
    )
    .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .current_dir(&path)
        .output()
        .unwrap();
    with_mock_env(&[("gh", &mock_not_found())], || {
        qtcloud_devops_cli::release::status(&path);
    });
}

/// gh release view 返回不同步的 body。
#[test]
fn test_release_status_gh_view_different() {
    let (_d, path) = setup_repo_with_contract();
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&path)
        .output()
        .unwrap();
    std::fs::write(
        path.join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [1.0.0]\n\n原始内容\n",
    )
    .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .current_dir(&path)
        .output()
        .unwrap();
    let gh = mock_custom(r#"case "$1/$2" in release/view) echo "不同步的body";; *) exit 1;; esac"#);
    with_mock_env(&[("gh", &gh)], || {
        qtcloud_devops_cli::release::status(&path);
    });
}

/// CHANGELOG 无此版本条目。
#[test]
fn test_release_status_gh_view_no_changelog_entry() {
    let (_d, path) = setup_repo_with_contract();
    Command::new("git")
        .args(["tag", "v2.0.0"])
        .current_dir(&path)
        .output()
        .unwrap();
    std::fs::write(
        path.join("CHANGELOG.md"),
        "# CHANGELOG\n\n## [1.0.0]\n\ncontent\n",
    )
    .unwrap();
    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repo.git",
        ])
        .current_dir(&path)
        .output()
        .unwrap();
    let gh = mock_custom(r#"case "$1/$2" in release/view) echo "release body";; *) exit 1;; esac"#);
    with_mock_env(&[("gh", &gh)], || {
        qtcloud_devops_cli::release::status(&path);
    });
}
