use std::path::Path;

use crate::contract;
use crate::test::Coverage;

/// 收集覆盖率数据。
///
/// 按语言读取对应的覆盖率报告。
pub(crate) fn collect_coverage(dir: &Path, lang: &contract::Language, threshold: f64) -> Coverage {
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
pub(crate) fn parse_lcov_coverage(content: &str) -> Option<f64> {
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
pub(crate) fn parse_cobertura_coverage(content: &str) -> Option<f64> {
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
