/// ROADMAP.md 格式常量和规则。

/// 格式问题。
#[derive(Debug)]
pub struct Issue {
    pub line: usize,
    pub scope: String,
    pub message: String,
}

/// 标准分类标题（PascalCase）。
pub const CATEGORIES: &[&str] = &[
    "### Added",
    "### Changed",
    "### Fixed",
    "### Removed",
    "### Deprecated",
    "### Security",
];

/// 检测一行是否为规划文件中的章节标题。
///
/// 支持格式：
/// - `## [X.Y.Z]` — ROADMAP.md 版本标题
/// - `## N. title` — TODO.md 序号标题
/// - 以上格式的可选后缀（如 `— 已发布`）
pub fn is_version_line(line: &str) -> Option<String> {
    let t = line.trim();
    if let Some(end) = t.find(']') {
        if t.starts_with("## [") {
            let ver = t["## [".len()..end].trim().trim_start_matches('v');
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    if t.starts_with("## ") {
        let rest = &t[3..].trim();
        if let Some(dot) = rest.find('.') {
            let num = &rest[..dot].trim();
            if num.chars().all(|c| c.is_ascii_digit()) && !num.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

pub fn rule_nonstandard_header(
    trimmed: &str,
    raw_line: &str,
    line_num: usize,
    scope: &str,
) -> Option<(String, Option<Issue>)> {
    if trimmed.starts_with("## ") && !is_version_line(trimmed).is_some() {
        Some((
            raw_line.to_string(),
            Some(Issue {
                line: line_num,
                scope: scope.to_string(),
                message: format!("非标准版本头（应为 ## [X.Y.Z]）: {}", trimmed),
            }),
        ))
    } else {
        None
    }
}

pub fn rule_nonstandard_category(
    trimmed: &str,
    raw_line: &str,
    line_num: usize,
    scope: &str,
) -> Option<(String, Option<Issue>)> {
    if trimmed.starts_with("### ")
        && !CATEGORIES
            .iter()
            .any(|c| c.to_lowercase() == trimmed.to_lowercase())
    {
        Some((
            raw_line.to_string(),
            Some(Issue {
                line: line_num,
                scope: scope.to_string(),
                message: format!("非标准分类标题: {}", trimmed),
            }),
        ))
    } else {
        None
    }
}

pub fn rule_v_prefix(
    trimmed: &str,
    raw_line: &str,
    _line_num: usize,
    scope: &str,
) -> Option<(String, Option<Issue>)> {
    if let Some(ver) = is_version_line(trimmed) {
        let raw_ver = trimmed
            .trim_start_matches("## [")
            .split(']')
            .next()
            .unwrap_or("")
            .trim();
        return Some(if raw_ver.starts_with('v') {
            let suffix = trimmed.split(']').nth(1).unwrap_or("");
            (
                format!("## [{}]{}", ver, suffix),
                Some(Issue {
                    line: _line_num,
                    scope: scope.to_string(),
                    message: format!("修复 v 前缀: {} → {}", raw_ver, ver),
                }),
            )
        } else {
            (raw_line.to_string(), None)
        });
    }
    None
}

pub fn rule_category_case(
    trimmed: &str,
    raw_line: &str,
    line_num: usize,
    scope: &str,
) -> Option<(String, Option<Issue>)> {
    if trimmed.starts_with("### ") {
        let lowered = trimmed.to_lowercase();
        if let Some(standard) = CATEGORIES.iter().find(|c| c.to_lowercase() == lowered) {
            if trimmed != *standard {
                let indent = &raw_line[..raw_line.len() - raw_line.trim_start().len()];
                return Some((
                    format!("{}{}", indent, standard),
                    Some(Issue {
                        line: line_num,
                        scope: scope.to_string(),
                        message: format!("修复大小写: {} → {}", trimmed, standard),
                    }),
                ));
            }
        }
        return Some((raw_line.to_string(), None));
    }
    None
}

pub fn rule_checkbox_format(
    trimmed: &str,
    _raw_line: &str,
    line_num: usize,
    scope: &str,
) -> Option<(String, Option<Issue>)> {
    let has_any_box = trimmed.contains("[x]") || trimmed.contains("[X]") || trimmed.contains("[ ]");
    let is_standard = trimmed.starts_with("- [x] ")
        || trimmed.starts_with("- [X] ")
        || trimmed.starts_with("- [ ] ");
    if has_any_box && !is_standard {
        let content_start = trimmed.find(']').map(|p| p + 1).unwrap_or(trimmed.len());
        let item_content = trimmed[content_start..].trim();
        let is_done = trimmed.contains("[x]") || trimmed.contains("[X]");
        let prefix = if is_done { "- [x]" } else { "- [ ]" };
        Some((
            format!("{} {}", prefix, item_content),
            Some(Issue {
                line: line_num,
                scope: scope.to_string(),
                message: format!(
                    "修复 checkbox 格式: {} → {} {}",
                    trimmed, prefix, item_content
                ),
            }),
        ))
    } else {
        None
    }
}

/// 对单行依次应用所有规则，返回(处理后的行, 可选问题)。
pub fn apply_rules_to_line(
    trimmed: &str,
    raw_line: &str,
    line_num: usize,
    scope: &str,
) -> (String, Option<Issue>) {
    if let Some(result) = rule_nonstandard_header(trimmed, raw_line, line_num, scope) {
        return result;
    }
    if let Some(result) = rule_nonstandard_category(trimmed, raw_line, line_num, scope) {
        return result;
    }
    if let Some(result) = rule_v_prefix(trimmed, raw_line, line_num, scope) {
        return result;
    }
    if let Some(result) = rule_category_case(trimmed, raw_line, line_num, scope) {
        return result;
    }
    if let Some(result) = rule_checkbox_format(trimmed, raw_line, line_num, scope) {
        return result;
    }
    (raw_line.to_string(), None)
}
