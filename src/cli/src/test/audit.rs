use std::path::{Path, PathBuf};

use crate::contract;
use crate::test::{is_io_fn, AuditReport};

/// 质量审计：扫描测试质量并对照门禁评估。
pub fn audit(repo_path: &Path, c: &contract::Contract, all: bool, verbose: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| repo_path.to_path_buf());

    // 收集所有 scope 目录
    let scope_dirs: Vec<(String, PathBuf)> = if all {
        if c.scopes.is_empty() {
            vec![("(root)".into(), repo_path.to_path_buf())]
        } else {
            c.scopes.iter().map(|s| (s.name.clone(), repo_path.join(&s.dir))).collect()
        }
    } else {
        let current = if let Some(s) = c.find_scope_by_path(repo_path, &cwd) {
            (s.name.clone(), repo_path.join(&s.dir))
        } else {
            ("(root)".into(), repo_path.to_path_buf())
        };
        vec![current]
    };

    println!("测试质量审计\n{}", "-".repeat(50));
    let mut all_met = true;
    let mut total_tests = 0u32;
    for (name, dir) in &scope_dirs {
        let report = scan_scope(dir, c, name)?;
        print_scope_audit(name, &report, verbose);
        check_missing_test_files(dir, name);
        total_tests += report.total_tests as u32;
        if !report.gates_met {
            all_met = false;
        }
    }

    println!("\n{}", "-".repeat(50));
    if total_tests == 0 {
        println!("  未检测到测试");
    } else {
        println!("  测试总数: {}", total_tests);
    }
    if !all_met {
        return Err("门禁未达标，请补充测试覆盖".into());
    }
    Ok(())
}

/// 扫描单个 scope 的测试质量。
fn scan_scope(dir: &Path, c: &contract::Contract, scope_name: &str) -> Result<AuditReport, String> {
    let rs_files = collect_source_files(dir);
    let mut test_fns = Vec::new();
    let mut pub_fns = Vec::new();
    let mut error_enums: Vec<(String, Vec<String>)> = Vec::new();
    let mut test_body = String::new();

    parse_source_files(&rs_files, &mut test_fns, &mut pub_fns, &mut error_enums, &mut test_body);

    let report = compute_coverage_report(&pub_fns, &error_enums, &test_body, &test_fns);
    Ok(evaluate_gates(report, dir, c, scope_name))
}

fn collect_source_files(dir: &Path) -> Vec<PathBuf> {
    let mut rs_files = Vec::new();
    collect_rs_files(dir, &mut rs_files);
    let tests_dir = dir.join("tests");
    if tests_dir.is_dir() { collect_rs_files(&tests_dir, &mut rs_files); }
    rs_files
}

fn parse_source_files(
    rs_files: &[PathBuf],
    test_fns: &mut Vec<String>,
    pub_fns: &mut Vec<(String, String)>,
    error_enums: &mut Vec<(String, Vec<String>)>,
    test_body: &mut String,
) {
    for f in rs_files {
        let content = std::fs::read_to_string(f).unwrap_or_default();
        collect_test_fns(&content, test_fns);
        collect_pub_fns(&content, pub_fns, f);
        collect_error_variants(&content, error_enums);
        if content.contains("#[cfg(test)]") || f.to_string_lossy().contains("/tests/") {
            test_body.push_str(&content);
            test_body.push('\n');
        }
    }
}

fn compute_coverage_report(
    pub_fns: &[(String, String)],
    error_enums: &[(String, Vec<String>)],
    test_body: &str,
    test_fns: &[String],
) -> AuditReport {
    let mut report = AuditReport::default();
    report.total_tests = test_fns.len();
    report.total_pub_fns = pub_fns.len();
    report.pure_pub_fns = pub_fns.iter().filter(|(name, _)| !is_io_fn(name)).count();
    report.tested_pub_fns = pub_fns.iter().filter(|(name, _)| test_body.contains(name.as_str())).count();

    report.uncovered_fns = pub_fns.iter()
        .filter(|(name, _)| !test_body.contains(name.as_str()))
        .map(|(name, path)| {
            let kind = if is_io_fn(name) { "I/O" } else { "纯函数" };
            (format!("{}::{}", path, name), kind.into())
        })
        .collect();

    report.error_variants = error_enums.iter().map(|(_, vs)| vs.len()).sum();
    report.tested_variants = 0;
    report.uncovered_variants.clear();
    for (enum_name, variants) in error_enums {
        for v in variants {
            let qualified = format!("{}::{}", enum_name, v);
            if test_body.contains(&qualified) { report.tested_variants += 1; }
            else { report.uncovered_variants.push(qualified); }
        }
    }
    report.uncovered_variants.sort();
    report
}

fn evaluate_gates(mut report: AuditReport, dir: &Path, c: &contract::Contract, scope_name: &str) -> AuditReport {
    let threshold = c.scopes.iter().find(|s| s.name == scope_name)
        .map(|s| c.scope_test_threshold(s)).unwrap_or(c.stages.test.threshold);
    report.coverage_threshold = threshold;
    let lang = contract::detect_languages(dir).into_iter().next()
        .unwrap_or(contract::Language::Unknown(String::new()));
    let coverage = crate::test::coverage::collect_coverage(dir, &lang, threshold);
    report.coverage_pct = coverage.percentage;

    let fn_ok = report.total_pub_fns == 0 || report.tested_pub_fns as f64 / report.total_pub_fns as f64 >= 0.5;
    let err_ok = report.error_variants == 0 || report.tested_variants as f64 / report.error_variants as f64 >= 0.5;
    let cov_ok = report.coverage_pct == 0.0 || report.coverage_pct >= report.coverage_threshold;
    report.gates_met = fn_ok && err_ok && cov_ok;
    report
}

/// 收集目录下所有 .rs 文件（排除 target/）。
pub(crate) fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
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

/// 从源码中收集 #[test] 函数名和 #[cfg(test)] 模块中的函数。
pub(crate) fn collect_test_fns(content: &str, fns: &mut Vec<String>) {
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

/// 从源码中收集 pub fn 名称，排除 macro_export、trait 内的。
pub(crate) fn collect_pub_fns(content: &str, fns: &mut Vec<(String, String)>, path: &Path) {
    let rel = path.to_string_lossy().to_string();
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("pub fn ") {
            if let Some(end) = rest.find('(') {
                let name = rest[..end].trim().to_string();
                if !name.is_empty() && !name.starts_with('_') && !name.contains(" ") {
                    fns.push((name, rel.clone()));
                }
            }
        }
    }
}

/// 检测行是否为错误枚举定义（`pub enum ...Error`），返回枚举名。
pub(crate) fn is_error_enum_declaration(line: &str) -> Option<String> {
    let t = line.trim();
    let rest = t.strip_prefix("pub enum ")?;
    let name = rest
        .split(|c: char| c == '{' || c == ' ' || c == '\t')
        .next()
        .unwrap_or("")
        .to_string();
    if name.contains("Error") || name.contains("error") {
        Some(name)
    } else {
        None
    }
}

/// 从 enum 体的一行中提取变体名（取第一个标识符，忽略元组参数、属性等）。
pub(crate) fn extract_variant_name(line: &str) -> Option<String> {
    let t = line.trim();
    if t.is_empty() || t.starts_with("//") || t.starts_with("#[") {
        return None;
    }
    let name = t
        .split(|c: char| c == '(' || c == '{' || c == ',' || c == ' ' || c == '\t')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() || name.starts_with('#') {
        None
    } else {
        Some(name)
    }
}

/// 从源码中收集错误枚举变体（基于 brace depth 的稳健解析）。
pub(crate) fn collect_error_variants(content: &str, enums: &mut Vec<(String, Vec<String>)>) {
    let mut depth: isize = 0;
    let mut enum_name = String::new();
    let mut variants: Vec<String> = Vec::new();
    let mut in_error_enum = false;
    let mut in_attr = false;

    for line in content.lines() {
        let t = line.trim();

        // 扫描模式：找 pub enum ...Error
        if !in_error_enum {
            if let Some(name) = is_error_enum_declaration(line) {
                enum_name = name;
                in_error_enum = true;
                variants.clear();
                depth = 0;
                in_attr = false;
                if t.contains('{') { depth = 1; }
            }
            continue;
        }

        // 收集模式：处理 enum 体内的行
        // 属性行
        if t.starts_with('#') {
            in_attr = true;
            if t.ends_with(']') { in_attr = false; }
            if t.contains('{') { depth += t.matches('{').count() as isize; }
            if t.contains('}') { depth -= t.matches('}').count() as isize; }
            if depth <= 0 { break; }
            continue;
        }
        if in_attr && t.ends_with(']') { in_attr = false; continue; }
        if in_attr { continue; }

        // 跟踪 brace depth
        if t.contains('{') { depth += t.matches('{').count() as isize; }
        if t.contains('}') { depth -= t.matches('}').count() as isize; }

        if depth <= 0 {
            if !variants.is_empty() {
                enums.push((enum_name.clone(), variants.clone()));
            }
            in_error_enum = false;
            continue;
        }

        if let Some(v) = extract_variant_name(t) {
            variants.push(v);
        }
    }

    // 文件终止时 enum 未关闭则保存
    if in_error_enum && !variants.is_empty() {
        enums.push((enum_name, variants));
    }
}

/// 输出单个 scope 的审查报告。
pub(crate) fn print_scope_audit(name: &str, report: &AuditReport, verbose: bool) {
    println!("\n  [{}]", name);
    print_test_count(report.total_tests);
    print_fn_coverage(report, verbose);
    print_error_coverage(report, verbose);
    print_line_coverage(report);
}

fn print_test_count(total_tests: usize) {
    println!("    测试函数:     {}", total_tests);
}

fn print_fn_coverage(report: &AuditReport, verbose: bool) {
    if report.total_pub_fns == 0 {
        return;
    }
    let covered = report.tested_pub_fns.min(report.total_pub_fns);
    let total = report.total_pub_fns;
    let pct = covered as f64 / total as f64 * 100.0;
    let icon = if pct >= 70.0 { "✅" } else { "⚠" };
    println!("    函数覆盖率:   {:.0}% ({}/{}) {} (纯函数 {} 个)", pct, covered, total, icon, report.pure_pub_fns);
    if verbose && !report.uncovered_fns.is_empty() {
        println!("      未覆盖函数:");
        for (path, kind) in &report.uncovered_fns {
            println!("        {} [{}]", path, kind);
        }
    }
}

fn print_error_coverage(report: &AuditReport, verbose: bool) {
    if report.error_variants == 0 {
        return;
    }
    let pct = report.tested_variants as f64 / report.error_variants as f64 * 100.0;
    let icon = if pct >= 80.0 { "✅" } else { "⚠" };
    println!("    错误变体覆盖: {:.0}% ({}/{}) {}", pct, report.tested_variants, report.error_variants, icon);
    if verbose && !report.uncovered_variants.is_empty() {
        println!("      未覆盖: {}", report.uncovered_variants.join(", "));
    }
}

fn print_line_coverage(report: &AuditReport) {
    if report.coverage_pct <= 0.0 {
        return;
    }
    let icon = if report.coverage_pct >= report.coverage_threshold { "✅" } else { "⚠" };
    println!("    行覆盖率:     {:.1}% (阈值 {}%) {}", report.coverage_pct, report.coverage_threshold, icon);
}

/// 检查 scope 内源文件是否缺少对应测试文件。
fn check_missing_test_files(dir: &Path, _scope_name: &str) {
    let mut rs_files = Vec::new();
    collect_rs_files(dir, &mut rs_files);

    let mut missing: Vec<String> = Vec::new();

    for file in &rs_files {
        let rel = file.strip_prefix(dir).unwrap_or(file);
        let file_name = rel.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // 跳过测试文件本身
        if file_name.ends_with("_test.rs") {
            continue;
        }
        // 跳过骨架文件
        if file_name == "build.rs" {
            continue;
        }
        if matches!(file_name, "mod.rs" | "lib.rs") {
            if let Ok(content) = std::fs::read_to_string(file) {
                if is_declaration_only(&content) {
                    continue;
                }
            }
        }

        // 检查是否有内联测试
        if let Ok(content) = std::fs::read_to_string(file) {
            if content.contains("#[cfg(test)]") {
                continue;
            }
        }

        // 检查是否有外部测试文件
        if let Some(stem) = rel.file_stem().and_then(|s| s.to_str()) {
            let test_path = dir.join("tests").join(format!("{}.rs", stem));
            if test_path.exists() {
                continue;
            }
        }

        missing.push(rel.to_string_lossy().to_string());
    }

    if missing.is_empty() {
        println!("    ✅ 全部源文件有对应测试");
    } else {
        println!("    ❌ 缺测试文件 ({} 个):", missing.len());
        for f in &missing {
            println!("      {}", f);
        }
    }
}

/// 检查文件是否仅包含声明（mod/use），不含函数/结构体/trait 定义。
fn is_declaration_only(content: &str) -> bool {
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') || t.starts_with("/*") || t.starts_with("*") {
            continue;
        }
        if t.starts_with("fn ") || t.starts_with("pub fn ")
            || t.starts_with("struct ") || t.starts_with("pub struct ")
            || t.starts_with("enum ") || t.starts_with("pub enum ")
            || t.starts_with("trait ") || t.starts_with("pub trait ")
            || t.starts_with("impl ") || t.starts_with("pub impl ") || t.starts_with("unsafe impl ")
        {
            return false;
        }
    }
    true
}
