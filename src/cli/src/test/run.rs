use std::path::Path;

use crate::contract;

/// 在 Docker 容器中运行测试和覆盖率。
///
/// 容器隔离编译环境，崩溃不影响宿主机。
/// 覆盖率报告写入容器挂载目录，`test status` 可直接读取。
/// 运行测试和覆盖率。
///
/// 在宿主机直接执行。如需容器隔离请使用 `scripts/test-in-container.sh`。
pub fn run(repo_path: &Path) -> Result<(), String> {
    println!("  运行测试...");
    run_direct(repo_path)
}

/// 直接在宿主机运行测试 + 覆盖率。
fn run_direct(repo_path: &Path) -> Result<(), String> {
    let c = crate::contract::load(repo_path);
    let cwd = std::env::current_dir().unwrap_or_else(|_| repo_path.to_path_buf());
    let scopes: Vec<_> = c
        .scopes
        .iter()
        .filter(|s| {
            let scope_abs = repo_path.join(&s.dir);
            cwd.starts_with(&scope_abs) || scope_abs.starts_with(&cwd)
        })
        .collect();

    let run_scoped = |dir: &Path, lang: &contract::Language| -> Result<(), String> {
        match run_coverage_for_lang(dir, lang) {
            Ok(true) => Ok(()),
            // 覆盖率失败但测试可能已通过，回退到只跑测试
            Ok(false) => {
                let summary = crate::test::summary::collect_test_summary_from_run(dir, lang)?;
                crate::test::summary::save_test_summary(dir, &summary);
                Ok(())
            }
            Err(e) => {
                // cargo llvm-cov 可能测试通过但覆盖率生成失败
                let summary = crate::test::summary::collect_test_summary_from_run(dir, lang)
                    .unwrap_or_default();
                if summary.failed == 0 && summary.total > 0 {
                    crate::test::summary::save_test_summary(dir, &summary);
                    return Ok(());
                }
                Err(e)
            }
        }
    };

    if scopes.is_empty() {
        let lang = crate::contract::detect_languages(repo_path)
            .into_iter()
            .next()
            .unwrap_or(contract::Language::Unknown(String::new()));
        run_scoped(repo_path, &lang)?;
    } else {
        for scope in &scopes {
            let scope_dir = repo_path.join(&scope.dir);
            if !scope_dir.exists() {
                println!("  [{}]     ⚠ 目录不存在，跳过", scope.name);
                continue;
            }
            let lang = c.resolve_language(scope, &scope_dir);
            println!("  [{}] 运行测试...", scope.name);
            run_scoped(&scope_dir, &lang)?;
        }
    }
    println!("  ✅ 测试通过");
    Ok(())
}

/// 在 repo 树中向上查找 Dockerfile。
#[allow(dead_code)]
pub(crate) fn run_tests_for_lang(dir: &Path, lang: &contract::Language) -> Result<(), String> {
    let Some((cmd, args)) = test_command(lang) else {
        println!("  ⚠ 不支持的语言: {:?}，跳过", lang);
        return Ok(());
    };
    let status = std::process::Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("启动 {} 失败: {}", cmd, e))?;
    if status.success() {
        println!("  ✅ {} 测试通过", cmd);
        Ok(())
    } else {
        Err(format!("{} 测试失败", cmd))
    }
}

pub(crate) fn coverage_command(
    lang: &contract::Language,
) -> Option<(&'static str, &'static [&'static str])> {
    match lang {
        contract::Language::Rust => Some((
            "cargo",
            &[
                "llvm-cov",
                "--lcov",
                "--output-path",
                "target/coverage/lcov.info",
            ],
        )),
        contract::Language::Python => Some(("coverage", &["xml"])),
        contract::Language::Go => Some((
            "go",
            &["tool", "cover", "-html=coverage.out", "-o", "coverage.html"],
        )),
        contract::Language::Dart => Some(("flutter", &["test", "--coverage"])),
        contract::Language::TypeScript => Some(("npx", &["nyc", "--reporter=lcov", "npm", "test"])),
        contract::Language::Unknown(_) => None,
    }
}

// ── 以下函数原属于已移除的 test run 命令，保留供外部库使用者参考 ──
// 可安全删除，不影响 CLI 功能。

/// 从 /proc/meminfo 读取 MemAvailable (kB)，计算安全的并行编译 job 数。
/// 公式：jobs = max(1, min(CPU核数, MemAvailable_GB / 1.5))
fn safe_parallel_jobs() -> usize {
    let mem_kb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                if l.starts_with("MemAvailable:") {
                    l.split_whitespace().nth(1)?.parse::<usize>().ok()
                } else {
                    None
                }
            })
        })
        .unwrap_or(4_194_304);
    let mem_gb = mem_kb as f64 / 1_048_576.0;
    let jobs_from_mem = (mem_gb / 1.5).floor() as usize;
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    jobs_from_mem.max(1).min(cpus)
}

/// 为 Rust 构建 cargo llvm-cov 参数（含自动并行度限制）。
fn rust_coverage_args(jobs: usize) -> Vec<String> {
    let mut args = vec![
        "llvm-cov".to_string(),
        "--lcov".to_string(),
        "--output-path".to_string(),
        "target/coverage/lcov.info".to_string(),
    ];
    if jobs > 0 {
        args.push("-j".to_string());
        args.push(jobs.to_string());
    }
    args
}

/// 生成覆盖率。返回 Ok(true) 表示该命令已一并运行了测试（如 cargo llvm-cov），
/// 调用方可跳过单独的 run_tests_for_lang。Err 表示测试/覆盖率执行失败。
fn run_coverage_for_lang(dir: &Path, lang: &contract::Language) -> Result<bool, String> {
    let (cmd, args): (&str, Vec<String>) = match lang {
        contract::Language::Rust => ("cargo", rust_coverage_args(safe_parallel_jobs())),
        _ => {
            let Some((c, a)) = coverage_command(lang) else {
                println!("  ⚠ {:?} 覆盖率不可用，跳过", lang);
                return Ok(false);
            };
            (c, a.iter().map(|s| s.to_string()).collect())
        }
    };
    let handles_tests = matches!(lang, contract::Language::Rust);
    println!("  生成覆盖率 ({})...", cmd);
    match std::process::Command::new(cmd)
        .args(&args)
        .current_dir(dir)
        .status()
    {
        Ok(s) if s.success() => {
            println!("  ✅ 覆盖率已更新");
            Ok(handles_tests)
        }
        Ok(_) if handles_tests => Err(format!("{} 测试失败", cmd)),
        Ok(_) => {
            println!("  ⚠ 覆盖率生成失败（可忽略）");
            Ok(false)
        }
        Err(e) => {
            println!("  ⚠ 覆盖率工具不可用: {}（可忽略）", e);
            Ok(false)
        }
    }
}

/// 返回语言对应的测试命令和标签，None 表示不支持。
pub(crate) fn test_command(
    lang: &contract::Language,
) -> Option<(&'static str, &'static [&'static str])> {
    match lang {
        contract::Language::Rust => Some(("cargo", &["test"])),
        contract::Language::Python => Some(("python", &["-m", "pytest"])),
        contract::Language::Go => Some(("go", &["test", "./..."])),
        contract::Language::Dart => Some(("flutter", &["test"])),
        contract::Language::TypeScript => Some(("npm", &["test"])),
        contract::Language::Unknown(_) => None,
    }
}

/// 返回语言对应的清单文件名（存在验证用），None 表示不需要验证。
pub(crate) fn test_manifest_file(lang: &contract::Language) -> Option<&'static str> {
    match lang {
        contract::Language::Rust => Some("Cargo.toml"),
        contract::Language::Python => Some("pyproject.toml"),
        contract::Language::Go => Some("go.mod"),
        contract::Language::Dart => Some("pubspec.yaml"),
        contract::Language::TypeScript => Some("package.json"),
        contract::Language::Unknown(_) => None,
    }
}
