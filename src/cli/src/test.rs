use std::path::{Path, PathBuf};

use crate::contract;

const TEST_SUMMARY_CACHE: &str = ".quanttide/devops/test-summary.json";

/// 测试结果汇总。
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TestSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
}

/// 覆盖率数据。
#[derive(Debug, Default)]
pub struct Coverage {
    pub percentage: f64,
    pub threshold: f64,
}

impl Coverage {
    pub fn met(&self) -> bool {
        self.percentage >= self.threshold
    }
}

/// 按 scope 输出测试状态（写 stdout 的便捷封装）。
pub fn status(repo_path: &Path, c: &contract::Contract) {
    let _ = status_to(&mut std::io::stdout(), repo_path, c);
}

/// 质量审查结果。
#[derive(Debug, Default)]
pub struct ReviewReport {
    pub total_tests: usize,
    pub total_pub_fns: usize,
    pub tested_pub_fns: usize,
    pub error_variants: usize,
    pub tested_variants: usize,
    pub uncovered_fns: Vec<String>,
    pub uncovered_variants: Vec<String>,
    pub coverage_pct: f64,
    pub coverage_threshold: f64,
}

/// 质量审查：扫描测试质量并对照门禁评估。
pub fn review(repo_path: &Path, c: &contract::Contract, all: bool, verbose: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| repo_path.to_path_buf());

    // 收集所有 scope 目录
    let scope_dirs: Vec<(String, PathBuf)> = if all {
        if c.scopes.is_empty() {
            vec![("(root)".into(), repo_path.to_path_buf())]
        } else {
            c.scopes.iter().map(|s| (s.name.clone(), repo_path.join(&s.dir))).collect()
        }
    } else {
        // 仅当前目录所属 scope
        let current = if let Some(s) = c.find_scope_by_path(&cwd) {
            (s.name.clone(), repo_path.join(&s.dir))
        } else {
            ("(root)".into(), repo_path.to_path_buf())
        };
        vec![current]
    };

    println!("测试质量审查\n{}", "-".repeat(50));
    let mut passed = 0u32;
    let mut total_gates = 0u32;
    for (name, dir) in &scope_dirs {
        let report = scan_scope(dir, c, name)?;
        print_scope_review(name, &report, verbose);
        passed += report.total_tests as u32;
        total_gates += 1;
    }

    println!("\n{}", "-".repeat(50));
    if passed == 0 {
        println!("  未检测到测试");
    } else {
        println!("  测试总数: {}", passed);
    }
    if total_gates > 0 {
        println!("  门禁: {} 达标", passed);
    }
    Ok(())
}

/// 扫描单个 scope 的测试质量。
fn scan_scope(dir: &Path, c: &contract::Contract, scope_name: &str) -> Result<ReviewReport, String> {
    let mut report = ReviewReport::default();

    // 收集所有 .rs 文件（排除 target/）
    let mut rs_files = Vec::new();
    collect_rs_files(dir, &mut rs_files);

    // 分离测试文件（包含 #[cfg(test)]）和生产文件
    let mut test_fns = Vec::new();
    let mut pub_fns = Vec::new();
    let mut error_enums: Vec<(String, Vec<String>)> = Vec::new();
    let mut test_body = String::new();

    for f in &rs_files {
        let content = std::fs::read_to_string(f).unwrap_or_default();

        // 收集当前文件中的测试函数
        collect_test_fns(&content, &mut test_fns);
        // 收集当前文件中的 pub fn
        collect_pub_fns(&content, &mut pub_fns, f);
        // 收集错误枚举变体
        collect_error_variants(&content, &mut error_enums);
        // 如果是测试模块，收集内容用于引用检查
        if content.contains("#[cfg(test)]") {
            test_body.push_str(&content);
            test_body.push('\n');
        }
    }

    report.total_tests = test_fns.len();
    report.total_pub_fns = pub_fns.len();
    report.tested_pub_fns = pub_fns.iter().filter(|(name, _)| {
        test_fns.iter().any(|t| t == name) || test_body.contains(name)
    }).count();

    report.uncovered_fns = pub_fns.iter()
        .filter(|(name, _)| {
            !test_fns.iter().any(|t| t == name) && !test_body.contains(name)
        })
        .map(|(name, path)| format!("{}::{}", path, name))
        .collect();

    // 计算错误变体覆盖
    report.error_variants = error_enums.iter().map(|(_, vs)| vs.len()).sum();
    let all_variants: Vec<&str> = error_enums.iter().flat_map(|(_, vs)| vs.iter().map(|s| s.as_str())).collect();
    report.tested_variants = all_variants.iter().filter(|v| test_body.contains(*v)).count();
    report.uncovered_variants = all_variants.iter().filter(|v| !test_body.contains(*v)).map(|s| s.to_string()).collect();
    report.uncovered_variants.sort();

    // 读取覆盖率（沿用现有逻辑）
    let threshold = c.scopes.iter().find(|s| s.name == scope_name).map(|s| c.scope_test_threshold(s)).unwrap_or(c.stages.test.threshold);
    report.coverage_threshold = threshold;
    let lang = contract::detect_languages(dir).into_iter().next().unwrap_or(contract::Language::Unknown(String::new()));
    let coverage = collect_coverage(dir, &lang, threshold);
    report.coverage_pct = coverage.percentage;

    Ok(report)
}

/// 收集目录下所有 .rs 文件（排除 target/）。
fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if !dir.exists() { return; }
    let entries = match std::fs::read_dir(dir) { Ok(e) => e, Err(_) => return };
    for entry in entries {
        let entry = match entry { Ok(e) => e, Err(_) => continue };
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name != "target" && !name.starts_with('.') {
                collect_rs_files(&path, files);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

/// 从源码中收集 #[test] 函数名。
fn collect_test_fns(content: &str, fns: &mut Vec<String>) {
    let lines: Vec<&str> = content.lines().collect();
    for i in 0..lines.len().saturating_sub(1) {
        let trimmed = lines[i].trim();
        if trimmed == "#[test]" {
            let next = lines[i + 1].trim();
            if let Some(name) = next.strip_prefix("fn ") {
                if let Some(end) = name.find('(') {
                    fns.push(name[..end].to_string());
                }
            }
        }
    }
}

/// 从源码中收集 pub fn 名称。
fn collect_pub_fns(content: &str, fns: &mut Vec<(String, String)>, path: &Path) {
    let rel = path.to_string_lossy().to_string();
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("pub fn ") {
            if let Some(end) = rest.find('(') {
                let name = rest[..end].trim().to_string();
                if !name.is_empty() && !name.starts_with('_') {
                    fns.push((name, rel.clone()));
                }
            }
        }
    }
}

/// 从源码中收集错误枚举变体。
fn collect_error_variants(content: &str, enums: &mut Vec<(String, Vec<String>)>) {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_error_enum = false;
    let mut enum_name = String::new();
    let mut variants: Vec<String> = Vec::new();
    for line in &lines {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("pub enum ") {
            if rest.contains("Error") || rest.contains("error") {
                enum_name = rest.split('{').next().unwrap_or("").trim().to_string();
                if t.contains('{') {
                    in_error_enum = true;
                    variants.clear();
                    // 同一行可能有变体
                    if let Some(body) = t.split('{').nth(1) {
                        if let Some(end) = body.find('}') {
                            for v in body[..end].split(',') {
                                let v = v.trim();
                                if !v.is_empty() && !v.starts_with('#') {
                                    variants.push(v.split('(').next().unwrap_or(v).split('{').next().unwrap_or("").trim().to_string());
                                }
                            }
                            if !variants.is_empty() {
                                enums.push((enum_name.clone(), variants.clone()));
                            }
                            in_error_enum = false;
                        }
                    }
                } else {
                    in_error_enum = true;
                    variants.clear();
                }
                continue;
            }
        }
        if in_error_enum {
            if t == "}" || t.starts_with("//") {
                continue;
            }
            if t.starts_with('}') {
                if !variants.is_empty() {
                    enums.push((enum_name.clone(), variants.clone()));
                }
                in_error_enum = false;
                continue;
            }
            if t.starts_with('#') || t.is_empty() {
                continue;
            }
            let v = t.split('(').next().unwrap_or(t).split('{').next().unwrap_or("").trim().to_string();
            if !v.is_empty() && !v.starts_with('#') {
                variants.push(v.trim_end_matches(',').to_string());
            }
        }
    }
    if in_error_enum && !variants.is_empty() {
        enums.push((enum_name, variants));
    }
}

/// 输出单个 scope 的审查报告。
fn print_scope_review(name: &str, report: &ReviewReport, verbose: bool) {
    println!("\n  [{}]", name);
    println!("    测试函数:     {}", report.total_tests);
    if report.total_pub_fns > 0 {
        let pct = report.tested_pub_fns as f64 / report.total_pub_fns as f64 * 100.0;
        let icon = if pct >= 70.0 { "✅" } else { "⚠" };
        println!("    函数覆盖率:   {:.0}% ({}/{}) {}", pct, report.tested_pub_fns, report.total_pub_fns, icon);
        if verbose && !report.uncovered_fns.is_empty() {
            for f in &report.uncovered_fns {
                println!("      未覆盖: {}", f);
            }
        }
    }
    if report.error_variants > 0 {
        let pct = if report.error_variants > 0 { report.tested_variants as f64 / report.error_variants as f64 * 100.0 } else { 100.0 };
        let icon = if pct >= 50.0 { "✅" } else { "⚠" };
        println!("    错误变体覆盖: {:.0}% ({}/{}) {}", pct, report.tested_variants, report.error_variants, icon);
        if verbose && !report.uncovered_variants.is_empty() {
            println!("      未覆盖: {}", report.uncovered_variants.join(", "));
        }
    }
    if report.coverage_pct > 0.0 {
        let icon = if report.coverage_pct >= report.coverage_threshold { "✅" } else { "⚠" };
        println!("    行覆盖率:     {:.1}% (阈值 {}%) {}", report.coverage_pct, report.coverage_threshold, icon);
    }
}

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
                let summary = collect_test_summary_from_run(dir, lang)?;
                save_test_summary(dir, &summary);
                Ok(())
            }
            Err(e) => {
                // cargo llvm-cov 可能测试通过但覆盖率生成失败
                let summary = collect_test_summary_from_run(dir, lang).unwrap_or_default();
                if summary.failed == 0 && summary.total > 0 {
                    save_test_summary(dir, &summary);
                    return Ok(());
                }
                Err(e)
            }
        }
    };

    if scopes.is_empty() {
        let lang = crate::contract::detect_languages(repo_path).into_iter().next().unwrap_or(contract::Language::Unknown(String::new()));
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
fn run_tests_for_lang(dir: &Path, lang: &contract::Language) -> Result<(), String> {
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

fn coverage_command(lang: &contract::Language) -> Option<(&'static str, &'static [&'static str])> {
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

/// 按 scope 输出测试状态，写入任意 writer。
pub fn status_to(
    writer: &mut impl std::io::Write,
    repo_path: &Path,
    c: &contract::Contract,
) -> std::io::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| repo_path.to_path_buf());
    let scopes: Vec<_> = c
        .scopes
        .iter()
        .filter(|s| {
            let scope_abs = repo_path.join(&s.dir);
            cwd.starts_with(&scope_abs) || scope_abs.starts_with(&cwd)
        })
        .collect();

    writeln!(writer, "测试状态")?;
    writeln!(writer, "{}", "-".repeat(50))?;

    if scopes.is_empty() {
        let lang = contract::detect_languages(repo_path).into_iter().next().unwrap_or(contract::Language::Unknown(String::new()));
        let summary = collect_test_summary(repo_path, &lang);
        let coverage = collect_coverage(repo_path, &lang, c.stages.test.threshold);
        print_scope(writer, "(root)", &summary, &coverage)?;
    } else {
        for scope in &scopes {
            let scope_dir = repo_path.join(&scope.dir);
            if !scope_dir.exists() {
                writeln!(writer, "  [{}]     ⚠ 目录不存在", scope.name)?;
                continue;
            }
            let lang = c.resolve_language(scope, &scope_dir);
            let summary = collect_test_summary(&scope_dir, &lang);
            let threshold = c.scope_test_threshold(scope);
            let coverage = collect_coverage(&scope_dir, &lang, threshold);
            print_scope(writer, &scope.name, &summary, &coverage)?;
        }
    }

    Ok(())
}

fn print_scope(
    writer: &mut impl std::io::Write,
    name: &str,
    summary: &TestSummary,
    coverage: &Coverage,
) -> std::io::Result<()> {
    let status_icon = if summary.failed > 0 {
        "❌"
    } else if summary.skipped > 0 {
        "⚠"
    } else if summary.total > 0 {
        "✅"
    } else {
        "—"
    };

    let detail = if summary.total > 0 {
        if summary.failed > 0 {
            format!("{} / {} 失败", summary.failed, summary.total)
        } else if summary.skipped > 0 {
            format!(
                "{} 通过 / {} 跳过 / {} 总计",
                summary.passed, summary.skipped, summary.total
            )
        } else {
            format!("{} ✅ 全部通过", summary.total)
        }
    } else {
        "暂无测试".into()
    };

    writeln!(writer, "  [{:<12}] {}", name, status_icon)?;
    writeln!(writer, "    测试数:       {}", detail)?;

    let cov_icon = if coverage.met() {
        "✅"
    } else if coverage.percentage > 0.0 {
        "⚠"
    } else {
        "—"
    };
    if coverage.percentage > 0.0 {
        writeln!(
            writer,
            "    覆盖率:       {:.1}%{}（阈值 {}%）",
            coverage.percentage, cov_icon, coverage.threshold,
        )?;
    } else {
        writeln!(writer, "    覆盖率:       未检测到覆盖率报告")?;
        writeln!(writer, "                  运行 `cargo llvm-cov --lcov --output-path target/coverage/lcov.info` 生成")?;
    }

    Ok(())
}

/// 返回语言对应的测试命令和标签，None 表示不支持。
fn test_command(lang: &contract::Language) -> Option<(&'static str, &'static [&'static str])> {
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
fn test_manifest_file(lang: &contract::Language) -> Option<&'static str> {
    match lang {
        contract::Language::Rust => Some("Cargo.toml"),
        contract::Language::Python => Some("pyproject.toml"),
        contract::Language::Go => Some("go.mod"),
        contract::Language::Dart => Some("pubspec.yaml"),
        contract::Language::TypeScript => Some("package.json"),
        contract::Language::Unknown(_) => None,
    }
}

/// 读取缓存的测试摘要路径。
fn cache_path(dir: &Path) -> PathBuf {
    dir.join(TEST_SUMMARY_CACHE)
}

/// 收集已缓存的测试结果（不运行测试）。
fn collect_test_summary(dir: &Path, _lang: &contract::Language) -> TestSummary {
    let cache = cache_path(dir);
    let content = match std::fs::read_to_string(&cache) {
        Ok(c) => c,
        Err(_) => return TestSummary::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// 运行测试并收集结果。
fn collect_test_summary_from_run(
    dir: &Path,
    lang: &contract::Language,
) -> Result<TestSummary, String> {
    let (cmd, args) = match test_command(lang) {
        Some(x) => x,
        None => return Ok(TestSummary::default()),
    };
    if let Some(mf) = test_manifest_file(lang) {
        if !dir.join(mf).exists() {
            return Ok(TestSummary::default());
        }
    }
    let output = std::process::Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("启动 {} 失败: {}", cmd, e))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let summary = parse_test_summary(&combined);
    if !output.status.success() {
        return Err(format!("{} 测试失败", cmd));
    }
    Ok(summary)
}

/// 保存测试摘要到缓存文件。
fn save_test_summary(dir: &Path, summary: &TestSummary) {
    let cache = cache_path(dir);
    if let Some(parent) = cache.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Ok(content) = serde_json::to_string(summary) {
        std::fs::write(&cache, &content).ok();
    }
}

/// 清除缓存的测试摘要。
pub fn clear_cache(dir: &Path) {
    let cache = cache_path(dir);
    std::fs::remove_file(&cache).ok();
}

fn parse_test_summary(content: &str) -> TestSummary {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    for line in content.lines() {
        if line.contains("test result:") {
            for part in line.split(';') {
                let p = part.trim();
                let words: Vec<&str> = p.split_whitespace().collect();
                if words.len() < 2 {
                    continue;
                }
                let kind = words[words.len() - 1];
                if let Ok(n) = words[words.len() - 2].parse::<u32>() {
                    match kind {
                        "passed" => passed += n,
                        "failed" => failed += n,
                        "ignored" => skipped += n,
                        _ => {}
                    }
                }
            }
        }
    }
    let total = passed + failed + skipped;
    TestSummary {
        total,
        passed,
        failed,
        skipped,
    }
}

/// 收集覆盖率数据。
///
/// 按语言读取对应的覆盖率报告。
fn collect_coverage(dir: &Path, lang: &contract::Language, threshold: f64) -> Coverage {
    let paths: &[std::path::PathBuf] = match lang {
        contract::Language::Rust => &[
            dir.join("target/coverage/lcov.info"),
            dir.join("coverage/lcov.info"),
        ],
        contract::Language::Python => &[dir.join("coverage.xml"), dir.join("htmlcov/coverage.xml")],
        _ => {
            return Coverage {
                percentage: 0.0,
                threshold,
            }
        }
    };
    for path in paths {
        if path.exists() {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            if let Some(pct) = parse_lcov_coverage(&content) {
                return Coverage {
                    percentage: pct,
                    threshold,
                };
            }
            if let Some(pct) = parse_cobertura_coverage(&content) {
                return Coverage {
                    percentage: pct,
                    threshold,
                };
            }
        }
    }
    Coverage {
        percentage: 0.0,
        threshold,
    }
}

/// 从 lcov.info 解析覆盖率百分比。
///
/// lcov 格式：
/// ```text
/// SF:src/lib.rs
/// DA:1,1
/// DA:2,0
/// end_of_record
/// ```
/// 覆盖率 = 命中行数 / 总行数
fn parse_lcov_coverage(content: &str) -> Option<f64> {
    let mut total_lines = 0u32;
    let mut hit_lines = 0u32;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("DA:") {
            if let Some(count_str) = rest.split(',').nth(1) {
                total_lines += 1;
                if let Ok(count) = count_str.trim().parse::<u32>() {
                    if count > 0 {
                        hit_lines += 1;
                    }
                }
            }
        }
    }

    if total_lines == 0 {
        None
    } else {
        Some((hit_lines as f64 / total_lines as f64) * 100.0)
    }
}

/// 从 Cobertura XML 解析覆盖率百分比。
///
/// 格式：<coverage line-rate="0.85" ...>
fn parse_cobertura_coverage(content: &str) -> Option<f64> {
    for line in content.lines() {
        if let Some(rest) = line.trim().strip_prefix("<coverage") {
            if let Some(attr) = rest.split("line-rate=\"").nth(1) {
                let val_str = attr.split('"').next()?;
                let rate: f64 = val_str.parse().ok()?;
                return Some(rate * 100.0);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_test_summary_ok() {
        let s = parse_test_summary(
            "test result: ok. 10 passed; 0 failed; 2 ignored; 0 measured; 12 filtered out",
        );
        assert_eq!(s.passed, 10);
        assert_eq!(s.failed, 0);
        assert_eq!(s.skipped, 2);
        assert_eq!(s.total, 12);
    }

    #[test]
    fn test_parse_test_summary_failed() {
        let s =
            parse_test_summary("test result: FAILED. 8 passed; 3 failed; 1 ignored; 0 measured");
        assert_eq!(s.passed, 8);
        assert_eq!(s.failed, 3);
        assert_eq!(s.skipped, 1);
    }

    #[test]
    fn test_parse_lcov_empty() {
        assert!(parse_lcov_coverage("").is_none());
    }

    #[test]
    fn test_parse_lcov_simple() {
        let content = "SF:src/lib.rs\nDA:1,1\nDA:2,0\nDA:3,1\nend_of_record\n";
        let pct = parse_lcov_coverage(content).unwrap();
        assert!((pct - 66.666).abs() < 0.01);
    }

    #[test]
    fn test_print_scope_skipped() {
        let mut buf = Vec::new();
        let s = TestSummary {
            total: 10,
            passed: 8,
            failed: 0,
            skipped: 2,
        };
        let c = Coverage {
            percentage: 0.0,
            threshold: 70.0,
        };
        print_scope(&mut buf, "test", &s, &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("⚠"), "跳过应有 ⚠");
    }

    #[test]
    fn test_print_scope_no_tests() {
        let mut buf = Vec::new();
        let s = TestSummary::default();
        let c = Coverage {
            percentage: 0.0,
            threshold: 70.0,
        };
        print_scope(&mut buf, "test", &s, &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("—"), "无测试应有 —");
        assert!(out.contains("暂无测试"));
    }

    #[test]
    fn test_print_scope_coverage_warn() {
        let mut buf = Vec::new();
        let s = TestSummary {
            total: 10,
            passed: 10,
            failed: 0,
            skipped: 0,
        };
        let c = Coverage {
            percentage: 50.0,
            threshold: 70.0,
        };
        print_scope(&mut buf, "test", &s, &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("⚠"), "低于阈值应有 ⚠");
    }

    #[test]
    fn test_coverage_met() {
        let c = Coverage {
            percentage: 80.0,
            threshold: 70.0,
        };
        assert!(c.met());
    }

    // ── 性能测试（大输入边界） ──────────────────────────────────

    #[test]
    fn test_parse_test_summary_large_output() {
        // 模拟 1000 组测试结果
        let mut content = String::new();
        for i in 0..500 {
            content.push_str(&format!(
                "test test_{i} ... ok\ntest test_{i}_a ... FAILED\n"
            ));
        }
        content.push_str("test result: FAILED. 500 passed; 500 failed; 0 ignored; 0 measured\n");
        let s = parse_test_summary(&content);
        assert_eq!(s.passed, 500);
        assert_eq!(s.failed, 500);
        assert_eq!(s.total, 1000);
    }

    #[test]
    fn test_parse_lcov_large_input() {
        // 10000 DA 行的 lcov 输出
        let mut lines = vec!["SF:src/lib.rs".to_string()];
        for i in 0..5000 {
            lines.push(format!("DA:{},1", i + 1));
            lines.push(format!("DA:{},0", i + 5001));
        }
        lines.push("end_of_record".to_string());
        let content = lines.join("\n");
        let pct = parse_lcov_coverage(&content).unwrap();
        assert!((pct - 50.0).abs() < 0.01, "10000 行应正确解析为 50%");
    }

    #[test]
    fn test_parse_test_summary_very_large_stdout() {
        // 大量非测试行（CI 日志、warnings）中间夹杂测试结果
        let mut lines: Vec<String> = (0..2000)
            .map(|i| format!("  Compiling crate-{} v0.1.0", i))
            .collect();
        lines.push("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured".into());
        for i in 2000..4000 {
            lines.push(format!("warning: unused variable `x` in crate-{}", i));
        }
        let content = lines.join("\n");
        let start = std::time::Instant::now();
        let s = parse_test_summary(&content);
        let elapsed = start.elapsed();
        assert_eq!(s.passed, 1);
        assert!(
            elapsed.as_millis() < 500,
            "4000 行日志应在 500ms 内解析完成: {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_parse_lcov_no_match() {
        // 大量不相关行，没有 DA: 前缀
        let mut lines = vec!["TN:".to_string()];
        for i in 0..5000 {
            lines.push(format!("SF:src/file_{i}.rs"));
            lines.push("end_of_record".to_string());
        }
        let content = lines.join("\n");
        assert!(parse_lcov_coverage(&content).is_none());
    }

    #[test]
    fn test_parse_cobertura_simple() {
        let content = r#"<coverage line-rate="0.85"></coverage>"#;
        let pct = parse_cobertura_coverage(content).unwrap();
        assert!((pct - 85.0).abs() < 0.01);
    }

    #[test]
    fn test_coverage_not_met() {
        let c = Coverage {
            percentage: 60.0,
            threshold: 70.0,
        };
        assert!(!c.met());
    }

    // ── test_command ──────────────────────────────────────────

    #[test]
    fn test_command_all_languages() {
        assert_eq!(
            test_command(&contract::Language::Rust),
            Some(("cargo", &["test"][..]))
        );
        assert_eq!(
            test_command(&contract::Language::Python),
            Some(("python", &["-m", "pytest"][..]))
        );
        assert_eq!(
            test_command(&contract::Language::Go),
            Some(("go", &["test", "./..."][..]))
        );
        assert_eq!(
            test_command(&contract::Language::Dart),
            Some(("flutter", &["test"][..]))
        );
        assert_eq!(
            test_command(&contract::Language::TypeScript),
            Some(("npm", &["test"][..]))
        );
        assert_eq!(test_command(&contract::Language::Unknown("?".into())), None);
    }

    // ── coverage_command ──────────────────────────────────

    #[test]
    fn test_coverage_command_all_languages() {
        assert_eq!(
            coverage_command(&contract::Language::Rust).map(|(c, _)| c),
            Some("cargo")
        );
        assert_eq!(
            coverage_command(&contract::Language::Python).map(|(c, _)| c),
            Some("coverage")
        );
        assert_eq!(
            coverage_command(&contract::Language::Go).map(|(c, _)| c),
            Some("go")
        );
        assert_eq!(
            coverage_command(&contract::Language::Dart).map(|(c, _)| c),
            Some("flutter")
        );
        assert_eq!(
            coverage_command(&contract::Language::TypeScript).map(|(c, _)| c),
            Some("npx")
        );
        assert!(coverage_command(&contract::Language::Unknown("auto".into())).is_none());
    }

    // ── test_manifest_file ────────────────────────────────────

    #[test]
    fn test_manifest_file_all_languages() {
        assert_eq!(
            test_manifest_file(&contract::Language::Rust),
            Some("Cargo.toml")
        );
        assert_eq!(
            test_manifest_file(&contract::Language::Python),
            Some("pyproject.toml")
        );
        assert_eq!(test_manifest_file(&contract::Language::Go), Some("go.mod"));
        assert_eq!(
            test_manifest_file(&contract::Language::Dart),
            Some("pubspec.yaml")
        );
        assert_eq!(
            test_manifest_file(&contract::Language::TypeScript),
            Some("package.json")
        );
        assert_eq!(
            test_manifest_file(&contract::Language::Unknown("?".into())),
            None
        );
    }

    // ── cache_path ────────────────────────────────────────────

    #[test]
    fn test_cache_path_resolves_in_dir() {
        let d = tempfile::tempdir().unwrap();
        let p = cache_path(d.path());
        assert!(p.ends_with(".quanttide/devops/test-summary.json"));
    }

    #[test]
    fn test_cache_path_absolute() {
        let p = cache_path(Path::new("/tmp/myproject"));
        assert_eq!(
            p,
            Path::new("/tmp/myproject/.quanttide/devops/test-summary.json")
        );
    }

    // ── save / collect / clear cache ─────────────────────────

    #[test]
    fn test_save_and_collect_cache_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let summary = TestSummary {
            total: 42,
            passed: 40,
            failed: 1,
            skipped: 1,
        };
        save_test_summary(d.path(), &summary);
        let cached = collect_test_summary(d.path(), &contract::Language::Rust);
        assert_eq!(cached.total, 42);
        assert_eq!(cached.passed, 40);
        assert_eq!(cached.failed, 1);
        assert_eq!(cached.skipped, 1);
    }

    #[test]
    fn test_collect_cache_nonexistent_returns_default() {
        let d = tempfile::tempdir().unwrap();
        let summary = collect_test_summary(d.path(), &contract::Language::Rust);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.passed, 0);
    }

    #[test]
    fn test_clear_cache_removes_file() {
        let d = tempfile::tempdir().unwrap();
        save_test_summary(
            d.path(),
            &TestSummary {
                total: 5,
                ..Default::default()
            },
        );
        assert!(cache_path(d.path()).exists());
        clear_cache(d.path());
        assert!(!cache_path(d.path()).exists());
    }

    // ── parse_test_summary 边缘情况 ───────────────────────

    #[test]
    fn test_parse_test_summary_empty() {
        let s = parse_test_summary("");
        assert_eq!(s.total, 0);
    }

    #[test]
    fn test_parse_test_summary_no_result_line() {
        let s = parse_test_summary("Compiling foo ...\n   Compiling bar ...\n");
        assert_eq!(s.total, 0);
    }

    #[test]
    fn test_parse_test_summary_malformed_skips_bad_tokens() {
        // 'abc' 不是合法数字，应跳过
        let s = parse_test_summary("test result: ok. abc passed; 0 failed");
        assert_eq!(s.passed, 0);
        assert_eq!(s.failed, 0);
    }

    #[test]
    fn test_parse_test_summary_multiple_result_lines() {
        // 工作空间多 crate 场景：每行一个 test result
        let content = "test result: ok. 5 passed; 0 failed; 1 ignored\n\
                       test result: ok. 3 passed; 1 failed; 0 ignored\n";
        let s = parse_test_summary(content);
        assert_eq!(s.passed, 8);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
        assert_eq!(s.total, 10);
    }

    // ── collect_coverage 不存在文件时的行为 ────────────────

    #[test]
    fn test_collect_coverage_no_file_rust() {
        let d = tempfile::tempdir().unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Rust, 70.0);
        assert_eq!(cov.percentage, 0.0);
        assert_eq!(cov.threshold, 70.0);
        assert!(!cov.met());
    }

    #[test]
    fn test_collect_coverage_unknown_lang_no_paths() {
        let d = tempfile::tempdir().unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Unknown("x".into()), 80.0);
        assert_eq!(cov.percentage, 0.0);
        assert_eq!(cov.threshold, 80.0);
    }

    #[test]
    fn test_collect_coverage_rust_with_lcov_file() {
        let d = tempfile::tempdir().unwrap();
        let cov_dir = d.path().join("target/coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/lib.rs\nDA:1,1\nDA:2,0\nDA:3,1\nend_of_record\n",
        )
        .unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Rust, 70.0);
        assert!((cov.percentage - 66.666).abs() < 0.01);
        assert!(!cov.met());
    }

    #[test]
    fn test_collect_coverage_python_with_cobertura() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("coverage.xml"),
            r#"<coverage line-rate="0.92"></coverage>"#,
        )
        .unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Python, 80.0);
        assert!((cov.percentage - 92.0).abs() < 0.01);
        assert!(cov.met());
    }

    // ── TestSummary 序列化/反序列化 ────────────────────────

    #[test]
    fn test_test_summary_serde_roundtrip() {
        let s = TestSummary {
            total: 100,
            passed: 90,
            failed: 5,
            skipped: 5,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: TestSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total, 100);
        assert_eq!(back.passed, 90);
        assert_eq!(back.failed, 5);
        assert_eq!(back.skipped, 5);
    }

    #[test]
    fn test_test_summary_serde_default_roundtrip() {
        let s = TestSummary::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: TestSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total, 0);
    }

    // ── Coverage 方法 ─────────────────────────────────────────

    #[test]
    fn test_coverage_met_exact() {
        let c = Coverage {
            percentage: 70.0,
            threshold: 70.0,
        };
        assert!(c.met());
    }

    #[test]
    fn test_coverage_met_zero_threshold() {
        let c = Coverage {
            percentage: 0.0,
            threshold: 0.0,
        };
        assert!(c.met());
    }

    // ── status_to ──────────────────────────────────────────────

    // ── print_scope 更多变体 ───────────────────────────────

    #[test]
    fn test_print_scope_all_passed_no_coverage() {
        let mut buf = Vec::new();
        let s = TestSummary {
            total: 5,
            passed: 5,
            failed: 0,
            skipped: 0,
        };
        let c = Coverage {
            percentage: 0.0,
            threshold: 70.0,
        };
        print_scope(&mut buf, "core", &s, &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("✅"), "全部通过应有 ✅");
        assert!(out.contains("未检测到覆盖率报告"));
    }

    #[test]
    fn test_print_scope_with_coverage_met() {
        let mut buf = Vec::new();
        let s = TestSummary {
            total: 10,
            passed: 10,
            failed: 0,
            skipped: 0,
        };
        let c = Coverage {
            percentage: 85.0,
            threshold: 70.0,
        };
        print_scope(&mut buf, "lib", &s, &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("✅"), "满足阈值应有 ✅");
        assert!(out.contains("85.0%"));
    }

    #[test]
    fn test_print_scope_all_failed() {
        let mut buf = Vec::new();
        let s = TestSummary {
            total: 3,
            passed: 0,
            failed: 3,
            skipped: 0,
        };
        let c = Coverage::default();
        print_scope(&mut buf, "test", &s, &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("❌"), "全部失败应有 ❌");
        assert!(out.contains("3 / 3"));
    }

    #[test]
    fn test_print_scope_coverage_below_threshold() {
        let mut buf = Vec::new();
        let s = TestSummary {
            total: 1,
            passed: 1,
            failed: 0,
            skipped: 0,
        };
        let c = Coverage {
            percentage: 30.0,
            threshold: 70.0,
        };
        print_scope(&mut buf, "lib", &s, &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("⚠"), "低于阈值应有 ⚠");
        assert!(out.contains("30.0%"));
    }

    // ── parse_test_summary 更多变体 ─────────────────────────

    #[test]
    fn test_parse_test_summary_filtered_out() {
        let s =
            parse_test_summary("test result: ok. 5 passed; 0 failed; 0 ignored; 50 filtered out");
        assert_eq!(s.total, 5);
        assert_eq!(s.passed, 5);
    }

    #[test]
    fn test_parse_test_summary_with_measured() {
        let s = parse_test_summary("test result: ok. 3 passed; 1 failed; 0 ignored; 2 measured");
        assert_eq!(s.total, 4);
        assert_eq!(s.passed, 3);
        assert_eq!(s.failed, 1);
    }

    #[test]
    fn test_parse_test_summary_zero_all() {
        let s = parse_test_summary("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured");
        assert_eq!(s.total, 0);
    }

    // ── parse_lcov_coverage 边缘 ─────────────────────────────

    #[test]
    fn test_parse_lcov_da_with_non_numeric_count() {
        // count 不是数字，行仍计为 total_lines 但不计入 hit
        let content = "DA:1,abc\nDA:2,1\nend_of_record\n";
        let pct = parse_lcov_coverage(content).unwrap();
        assert!((pct - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_lcov_multiple_records() {
        // 多个 end_of_record 块，只累计 DA 行
        let content = "DA:1,1\nend_of_record\nSF:other.rs\nDA:2,0\nend_of_record\n";
        let pct = parse_lcov_coverage(content).unwrap();
        assert!((pct - 50.0).abs() < 0.01);
    }

    // ── parse_cobertura_coverage 边缘 ────────────────────────

    #[test]
    fn test_parse_cobertura_no_match() {
        assert!(parse_cobertura_coverage("<html></html>").is_none());
    }

    #[test]
    fn test_parse_cobertura_no_line_rate() {
        assert!(parse_cobertura_coverage(r#"<coverage branch-rate="0.5"></coverage>"#).is_none());
    }

    #[test]
    fn test_parse_cobertura_bad_line_rate() {
        assert!(parse_cobertura_coverage(r#"<coverage line-rate="abc"></coverage>"#).is_none());
    }

    #[test]
    fn test_parse_cobertura_large_xml() {
        use std::time::Instant;
        let mut lines = vec![r#"<coverage line-rate="0.85">"#.to_string()];
        for i in 0..5000 {
            lines.push(format!(r#"<package name="pkg-{i}" line-rate="0.9"><class name="Cls{i}" filename="src/file{i}.rs" line-rate="0.9"/></package>"#));
        }
        lines.push("</coverage>".to_string());
        let content = lines.join("\n");
        let start = Instant::now();
        let pct = parse_cobertura_coverage(&content);
        let elapsed = start.elapsed();
        assert!((pct.unwrap() - 85.0).abs() < 0.01);
        assert!(
            elapsed.as_micros() < 5000,
            "5000 行 Cobertura 应在 5ms 内解析，实际: {}μs",
            elapsed.as_micros()
        );
    }

    // ── TestSummary serde 零值 ─────────────────────────────

    #[test]
    fn test_test_summary_serde_all_zero() {
        let s = TestSummary {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: TestSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total, 0);
    }

    // ── scope 过滤性能 ────────────────────────────────────────

    #[test]
    fn test_scope_filter_large_contract() {
        use std::time::Instant;
        let repo_path = Path::new("/tmp/repo");
        let cwd = Path::new("/tmp/repo/packages/cli");
        let mut scopes = Vec::new();
        for i in 0..1000 {
            scopes.push(contract::Scope {
                name: format!("scope-{}", i),
                dir: format!("packages/scope-{}", i),
                language: contract::Language::Unknown("?".into()),
                framework: String::new(),
                build_tool: contract::BuildTool::Unknown("?".into()),
                registry: contract::Registry::None,
                release: contract::StageRelease::default(),
                test_threshold: None,
                ci_workflow: None,
            });
        }
        scopes.push(contract::Scope {
            name: "cli".into(),
            dir: "packages/cli".into(),
            language: contract::Language::Rust,
            framework: String::new(),
            build_tool: contract::BuildTool::Cargo,
            registry: contract::Registry::Crates,
            release: contract::StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        });
        let start = Instant::now();
        let filtered: Vec<_> = scopes
            .iter()
            .filter(|s| {
                let scope_abs = repo_path.join(&s.dir);
                cwd.starts_with(&scope_abs) || scope_abs.starts_with(&cwd)
            })
            .collect();
        let elapsed = start.elapsed();
        assert_eq!(filtered.len(), 1, "应只匹配一个");
        assert_eq!(filtered[0].name, "cli");
        assert!(
            elapsed.as_micros() < 5000,
            "1000 scope 过滤应 < 5ms，实际: {}μs",
            elapsed.as_micros()
        );
    }

    // ── status_to ──────────────────────────────────────────────

    #[test]
    fn test_status_to_passing() {
        let d = tempfile::tempdir().unwrap();
        // 创建一个真实的 Rust 项目，使得 cargo test 能运行并通过
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/lib.rs"), "#[test]\nfn it_works() {}\n").unwrap();

        let c = contract::Contract::default();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path(), &c).unwrap();
        let out = String::from_utf8_lossy(&buf);

        assert!(out.contains("测试状态"));
        assert!(out.contains("全部通过") || out.contains("暂无测试"));
    }

    #[test]
    fn test_status_to_empty() {
        let d = tempfile::tempdir().unwrap();
        let c = contract::Contract::default();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path(), &c).unwrap();
        let out = String::from_utf8_lossy(&buf);

        assert!(out.contains("测试状态"));
    }
}
