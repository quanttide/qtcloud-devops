use std::path::Path;

use crate::contract;

/// 测试结果汇总。
#[derive(Debug, Default)]
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

/// 按 scope 输出测试状态，写入任意 writer。
pub fn status_to(
    writer: &mut impl std::io::Write,
    repo_path: &Path,
    c: &contract::Contract,
) -> std::io::Result<()> {
    let scopes = &c.scopes;

    writeln!(writer, "测试状态")?;
    writeln!(writer, "{}", "-".repeat(50))?;

    if scopes.is_empty() {
        let lang = contract::detect_by_files(repo_path);
        let summary = collect_test_summary(repo_path, &lang);
        let coverage = collect_coverage(repo_path, &lang, c.stages.test.threshold);
        print_scope(writer, "(root)", &summary, &coverage)?;
    } else {
        for scope in scopes {
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

/// 收集测试结果。
///
/// 按语言运行对应的测试命令，解析输出。
fn collect_test_summary(dir: &Path, lang: &contract::Language) -> TestSummary {
    let (cmd, args) = match test_command(lang) {
        Some(x) => x,
        None => return TestSummary::default(),
    };
    if let Some(mf) = test_manifest_file(lang) {
        if !dir.join(mf).exists() {
            return TestSummary::default();
        }
    }
    let result = std::process::Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .output();
    match result {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            let errors = String::from_utf8_lossy(&o.stderr);
            // Rust 的输出在 stdout，pytest 的输出在 stderr
            let combined = format!("{}{}", output, errors);
            parse_test_summary(&combined)
        }
        Err(_) => TestSummary::default(),
    }
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
    fn test_coverage_met() {
        let c = Coverage {
            percentage: 80.0,
            threshold: 70.0,
        };
        assert!(c.met());
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
}
