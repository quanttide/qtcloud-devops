use std::path::{Path, PathBuf};

use crate::contract;
use crate::test::TestSummary;

const TEST_SUMMARY_CACHE: &str = ".quanttide/devops/test-summary.json";

/// 读取缓存的测试摘要路径。
pub(crate) fn cache_path(dir: &Path) -> PathBuf {
    dir.join(TEST_SUMMARY_CACHE)
}

/// 收集已缓存的测试结果（不运行测试）。
#[expect(dead_code)]
pub(crate) fn collect_test_summary(dir: &Path, _lang: &contract::Language) -> TestSummary {
    let cache = cache_path(dir);
    let content = match std::fs::read_to_string(&cache) {
        Ok(c) => c,
        Err(_) => return TestSummary::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// 运行测试并收集结果。
pub(crate) fn collect_test_summary_from_run(
    dir: &Path,
    lang: &contract::Language,
) -> Result<TestSummary, String> {
    let (cmd, args) = match crate::test::run::test_command(lang) {
        Some(x) => x,
        None => return Ok(TestSummary::default()),
    };
    if let Some(mf) = crate::test::run::test_manifest_file(lang) {
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
pub(crate) fn save_test_summary(dir: &Path, summary: &TestSummary) {
    let cache = cache_path(dir);
    if let Some(parent) = cache.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Ok(content) = serde_json::to_string(summary) {
        std::fs::write(&cache, &content).ok();
    }
}

/// 清除本地测试产物：覆盖率报告、缓存等。
pub fn clear_cache(dir: &Path) {
    let targets = [
        dir.join("target/coverage"),
        dir.join("coverage.xml"),
        dir.join("htmlcov"),
        dir.join(".quanttide/devops/test-summary.json"),
    ];
    for t in &targets {
        if t.is_dir() { std::fs::remove_dir_all(t).ok(); }
        else if t.exists() { std::fs::remove_file(t).ok(); }
    }
    println!("  ✓ 测试产物已清理");
}

pub(crate) fn parse_test_summary(content: &str) -> TestSummary {
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
