/// plan 命令：ROADMAP.md 规划管理。
///
/// 对应 `data/roadmap/platform/plan-command.md`。
///
/// 三个子命令：
/// - `status` — 查看 scope 规划进度
/// - `clean`  — 删除已完成条目
/// - `doctor` — 修复格式问题（规则修复 + LLM 修复）
use std::path::{Path, PathBuf};

use quanttide_devops::source::roadmap::{Roadmap, RoadmapVersion};

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl From<String> for PlanError {
    fn from(s: String) -> Self {
        PlanError::Other(s)
    }
}

impl From<quanttide_devops::source::roadmap::RoadmapError> for PlanError {
    fn from(e: quanttide_devops::source::roadmap::RoadmapError) -> Self {
        PlanError::Other(e.to_string())
    }
}

/// 检测一行是否为规划文件中的章节标题。
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
    // TODO.md 格式: ## N. title
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

/// 格式问题。
/// 验证发现的格式问题。
#[derive(Debug)]
pub struct Issue {
    pub line: usize,
    pub scope: String,
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════════════
// 路径解析
// ═══════════════════════════════════════════════════════════════════════

/// 解析 scope 参数，返回实际 ROADMAP.md 路径。
pub fn resolve_roadmap_path(repo_path: &Path, scope: Option<&str>) -> PathBuf {
    let c = crate::contract::load(repo_path);
    match scope {
        Some(name) if !name.is_empty() => {
            // 按 scope 名称查找
            if let Some(s) = c.scopes.iter().find(|s| s.name == name) {
                repo_path.join(&s.dir).join("ROADMAP.md")
            } else {
                // 回退：scope 名作为子目录
                repo_path.join(name).join("ROADMAP.md")
            }
        }
        _ => {
            // 省略 scope → 找当前目录所属 scope
            let current_dir = std::env::current_dir().unwrap_or_else(|_| repo_path.to_path_buf());
            if let Some(s) = c.find_scope_by_path(&current_dir) {
                repo_path.join(&s.dir).join("ROADMAP.md")
            } else {
                repo_path.join("ROADMAP.md")
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// plan status
// ═══════════════════════════════════════════════════════════════════════

/// 获取 scope 对应的规划目录（ROADMAP.md 和 TODO.md 所在目录）。
pub fn resolve_roadmap_dir(repo_path: &Path, scope: Option<&str>) -> PathBuf {
    let c = crate::contract::load(repo_path);
    match scope {
        Some(name) if !name.is_empty() => {
            if let Some(s) = c.scopes.iter().find(|s| s.name == name) {
                repo_path.join(&s.dir)
            } else {
                repo_path.join(name)
            }
        }
        _ => {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| repo_path.to_path_buf());
            if let Some(s) = c.find_scope_by_path(&current_dir) {
                repo_path.join(&s.dir)
            } else {
                repo_path.to_path_buf()
            }
        }
    }
}

/// 修复 ROADMAP 或 TODO 文件的格式。
pub fn doctor_file(path: &Path, scope: &str) -> Result<Vec<Issue>, PlanError> {
    edit_roadmap(path, scope)
}

/// 解析 ROADMAP.md，返回各版本进度列表。
pub fn parse_roadmap(path: &Path) -> Result<Vec<RoadmapVersion>, PlanError> {
    let roadmap = Roadmap::from_path(path)?;
    Ok(roadmap.versions().to_vec())
}

/// 解析 ROADMAP.md 字符串，返回各版本进度列表。
pub fn parse_roadmap_str(s: &str) -> Result<Vec<RoadmapVersion>, PlanError> {
    let roadmap = Roadmap::from_str(s)?;
    Ok(roadmap.versions().to_vec())
}

/// 格式化输出 scope 规划进度。
pub fn print_status(repo_path: &Path, scope: Option<&str>) -> Result<(), PlanError> {
    let mut stdout = std::io::stdout();
    print_status_to(&mut stdout, repo_path, scope)
}

/// 写入指定 writer 的版本（可测试）。
pub fn print_status_to(
    writer: &mut impl std::io::Write,
    repo_path: &Path,
    scope: Option<&str>,
) -> Result<(), PlanError> {
    let plan_dir = resolve_plan_dir(repo_path, scope);
    let scope_label = scope.unwrap_or("(auto)");
    let files = ["ROADMAP.md", "TODO.md"];
    let mut found = false;
    for fname in &files {
        let path = plan_dir.join(fname);
        if path.exists() {
            found = true;
            try_print_plan_file(writer, &path, fname, scope_label)?;
        }
    }
    if !found {
        writeln!(
            writer,
            "  未创建规划文件: {}",
            plan_dir.join("ROADMAP.md").display()
        )
        .ok();
    }
    Ok(())
}

/// 尝试打印单个规划文件的状态。
fn try_print_plan_file(
    writer: &mut impl std::io::Write,
    path: &Path,
    _label: &str,
    scope_label: &str,
) -> Result<(), PlanError> {
    let versions = parse_roadmap(path).unwrap_or_default();
    if versions.is_empty() {
        // 对于 TODO.md 空结果不是错误，跳过即可
        if _label == "TODO.md" {
            return Ok(());
        }
        writeln!(writer, "  未找到标准规划条目").ok();
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let has_unknown_headers = content.lines().any(|l| {
            let t = l.trim();
            (t.starts_with("## ") && !t.starts_with("## ["))
                || (t.starts_with("### ")
                    && !CATEGORIES
                        .iter()
                        .any(|c| c.to_lowercase() == t.to_lowercase()))
        });
        if has_unknown_headers {
            let settings = quanttide_agent::Settings::from_env();
            if !settings.llm_api_key.is_empty()
                && !settings.llm_base_url.is_empty()
                && !settings.llm_model.is_empty()
                && cfg!(not(test))
            {
                writeln!(writer, "  🔄 检测到非标准格式，调用 LLM 转换...").ok();
                if let Ok(llm_result) = edit_llm(&content, scope_label, &settings, path) {
                    if llm_result.is_some() {
                        if let Ok(new_versions) = parse_roadmap(path) {
                            if !new_versions.is_empty() {
                                return print_progress(writer, scope_label, &new_versions);
                            }
                        }
                    }
                }
            }
            writeln!(
                writer,
                "  ⚠ 文件含有非标准格式的标题，运行 `plan edit` 查看详情"
            )
            .ok();
        }
        return Ok(());
    }
    print_progress(writer, scope_label, &versions)
}

/// 返回 planning 文件所在目录。
fn resolve_plan_dir(repo_path: &Path, scope: Option<&str>) -> PathBuf {
    let p = resolve_roadmap_path(repo_path, scope);
    p.parent().unwrap_or(repo_path).to_path_buf()
}

/// 输出进度表（抽离以供 LLM 转换后重用）。
fn print_progress(
    writer: &mut impl std::io::Write,
    scope_label: &str,
    versions: &[RoadmapVersion],
) -> Result<(), PlanError> {
    writeln!(writer, "  [{}] 规划进度", scope_label).ok();
    writeln!(writer, "  {}", "-".repeat(40)).ok();

    let mut total_done = 0usize;
    let mut total_all = 0usize;

    for v in versions {
        let rate = if v.total > 0 {
            v.done as f64 / v.total as f64 * 100.0
        } else {
            0.0
        };
        writeln!(
            writer,
            "  [{:<8}] {:>2}/{:>2} 完成 ({:.0}%)",
            v.version, v.done, v.total, rate
        )
        .ok();
        total_done += v.done;
        total_all += v.total;
    }

    let overall = if total_all > 0 {
        total_done as f64 / total_all as f64 * 100.0
    } else {
        0.0
    };
    writeln!(writer, "  {}", "-".repeat(40)).ok();
    writeln!(
        writer,
        "  总计:  {}/{} 完成 ({:.0}%)",
        total_done, total_all, overall
    )
    .ok();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// plan clean
// ═══════════════════════════════════════════════════════════════════════

const CATEGORIES: &[&str] = &[
    "### Added",
    "### Changed",
    "### Fixed",
    "### Removed",
    "### Deprecated",
    "### Security",
    "### Refactor",
];

/// 删除规划文件中所有已完成条目（`- [x]` / `- [X]`）。
/// 通用函数，可用于 ROADMAP.md 和 TODO.md。
pub fn clean_done_items(path: &Path) -> Result<usize, PlanError> {
    let content = std::fs::read_to_string(path)?;
    let original_len = content.len();

    let mut lines: Vec<&str> = content.lines().collect();
    lines.retain(|l| {
        let t = l.trim();
        !t.starts_with("- [x]") && !t.starts_with("- [X]")
    });

    if lines.is_empty() {
        std::fs::write(path, "")?;
        return Ok(original_len);
    }

    // 清理尾部空行
    while let Some(last) = lines.last() {
        if last.trim().is_empty() {
            lines.pop();
        } else {
            break;
        }
    }

    let mut output = String::new();
    for line in &lines {
        output.push_str(line);
        output.push('\n');
    }
    std::fs::write(path, &output)?;
    Ok(original_len.saturating_sub(output.len()))
}

/// 删除 ROADMAP.md 中所有已完成条目，并级联清理空分类和空版本标题。
pub fn clean_roadmap(path: &Path) -> Result<usize, PlanError> {
    let removed = clean_done_items(path)?;

    let content = std::fs::read_to_string(path)?;
    let original_len = content.len();
    let mut lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(removed);
    }

    // 删除空的分类标题（跳过空行看后面是否真有内容）
    let mut i = 0;
    while i < lines.len() {
        if CATEGORIES.iter().any(|c| {
            let t = lines[i].trim();
            t == *c || t.eq_ignore_ascii_case(c)
        }) {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j >= lines.len()
                || CATEGORIES.iter().any(|c| {
                    let t = lines[j].trim();
                    t == *c || t.eq_ignore_ascii_case(c)
                })
                || is_version_line(lines[j]).is_some()
            {
                lines.remove(i);
                continue;
            }
        }
        i += 1;
    }

    // 第三遍：删除空的版本标题（跳过空行看后面是否真有内容）
    let mut i = 0;
    while i < lines.len() {
        if is_version_line(lines[i]).is_some() {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j >= lines.len() || is_version_line(lines[j]).is_some() {
                // 后面是文件尾或另一个版本头 → 此版本为空
                lines.remove(i);
                continue;
            }
            // 后面有内容（checkbox、分类等）→ 保留
        }
        i += 1;
    }
    if let Some(last) = lines.last() {
        if is_version_line(last).is_some() {
            lines.pop();
        }
    }

    // 清理尾部空行
    while let Some(last) = lines.last() {
        if last.trim().is_empty() {
            lines.pop();
        } else {
            break;
        }
    }

    if lines.is_empty() {
        std::fs::write(path, "")?;
        return Ok(removed + original_len);
    }

    let mut output = String::new();
    for line in &lines {
        output.push_str(line);
        output.push('\n');
    }
    std::fs::write(path, &output)?;
    Ok(removed + original_len.saturating_sub(output.len()))
}

// ═══════════════════════════════════════════════════════════════════════
// plan edit
// ═══════════════════════════════════════════════════════════════════════

/// 编辑 ROADMAP.md：读取原始格式 → 标准化（LLM + 规则）→ 写回。
pub fn edit_roadmap(path: &Path, scope: &str) -> Result<Vec<Issue>, PlanError> {
    let content = std::fs::read_to_string(path)?;

    let mut issues: Vec<Issue> = Vec::new();

    // Phase 1: LLM 先判断并修复（LLM 已配置且非测试环境时）
    let settings = quanttide_agent::Settings::from_env();
    if !settings.llm_api_key.is_empty() && !settings.llm_base_url.is_empty() && cfg!(not(test)) {
        if let Some(llm_issues) = edit_llm(&content, scope, &settings, path)? {
            issues.extend(llm_issues);
        }
    }

    // Phase 2: 规则校验（对 LLM 修复后的内容或原内容做细节修复）
    let content_after_llm = std::fs::read_to_string(path)?;
    let rule_issues = apply_rule_fixes(path, &content_after_llm, scope)?;
    issues.extend(rule_issues);

    Ok(issues)
}

/// 规则修复：v 前缀、分类大小写、checkbox 格式。
fn apply_rule_fixes(path: &Path, content: &str, scope: &str) -> Result<Vec<Issue>, PlanError> {
    let mut issues: Vec<Issue> = Vec::new();
    let mut new_lines: Vec<String> = Vec::new();

    for (idx, raw_line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = raw_line.trim();

        let (new_line, opt_issue) = apply_rules_to_line(trimmed, raw_line, line_num, scope);
        if let Some(issue) = opt_issue {
            issues.push(issue);
        }
        new_lines.push(new_line);
    }

    if !issues.is_empty() {
        write_lines(path, &new_lines)?;
    }
    Ok(issues)
}

/// 对单行依次应用所有规则，返回(处理后的行, 可选问题)。
fn apply_rules_to_line(trimmed: &str, raw_line: &str, line_num: usize, scope: &str) -> (String, Option<Issue>) {
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

fn rule_nonstandard_header(trimmed: &str, raw_line: &str, line_num: usize, scope: &str) -> Option<(String, Option<Issue>)> {
    if trimmed.starts_with("## ") && !is_version_line(trimmed).is_some() {
        Some((raw_line.to_string(), Some(Issue {
            line: line_num, scope: scope.to_string(),
            message: format!("非标准版本头（应为 ## [X.Y.Z]）: {}", trimmed),
        })))
    } else { None }
}

fn rule_nonstandard_category(trimmed: &str, raw_line: &str, line_num: usize, scope: &str) -> Option<(String, Option<Issue>)> {
    if trimmed.starts_with("### ")
        && !CATEGORIES.iter().any(|c| c.to_lowercase() == trimmed.to_lowercase())
    {
        Some((raw_line.to_string(), Some(Issue {
            line: line_num, scope: scope.to_string(),
            message: format!("非标准分类标题: {}", trimmed),
        })))
    } else { None }
}

fn rule_v_prefix(trimmed: &str, raw_line: &str, _line_num: usize, scope: &str) -> Option<(String, Option<Issue>)> {
    if let Some(ver) = is_version_line(trimmed) {
        let raw_ver = trimmed.trim_start_matches("## [").split(']').next().unwrap_or("").trim();
        return Some(if raw_ver.starts_with('v') {
            let suffix = trimmed.split(']').nth(1).unwrap_or("");
            (format!("## [{}]{}", ver, suffix), Some(Issue {
                line: _line_num, scope: scope.to_string(),
                message: format!("修复 v 前缀: {} → {}", raw_ver, ver),
            }))
        } else {
            (raw_line.to_string(), None)
        });
    }
    None
}

fn rule_category_case(trimmed: &str, raw_line: &str, line_num: usize, scope: &str) -> Option<(String, Option<Issue>)> {
    if trimmed.starts_with("### ") {
        let lowered = trimmed.to_lowercase();
        if let Some(standard) = CATEGORIES.iter().find(|c| c.to_lowercase() == lowered) {
            if trimmed != *standard {
                let indent = &raw_line[..raw_line.len() - raw_line.trim_start().len()];
                return Some((format!("{}{}", indent, standard), Some(Issue {
                    line: line_num, scope: scope.to_string(),
                    message: format!("修复大小写: {} → {}", trimmed, standard),
                })));
            }
        }
        return Some((raw_line.to_string(), None));
    }
    None
}

fn rule_checkbox_format(trimmed: &str, _raw_line: &str, line_num: usize, scope: &str) -> Option<(String, Option<Issue>)> {
    let has_any_box = trimmed.contains("[x]") || trimmed.contains("[X]") || trimmed.contains("[ ]");
    let is_standard = trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") || trimmed.starts_with("- [ ] ");
    if has_any_box && !is_standard {
        let content_start = trimmed.find(']').map(|p| p + 1).unwrap_or(trimmed.len());
        let item_content = trimmed[content_start..].trim();
        let is_done = trimmed.contains("[x]") || trimmed.contains("[X]");
        let prefix = if is_done { "- [x]" } else { "- [ ]" };
        Some((format!("{} {}", prefix, item_content), Some(Issue {
            line: line_num, scope: scope.to_string(),
            message: format!("修复 checkbox 格式: {} → {} {}", trimmed, prefix, item_content),
        })))
    } else { None }
}

/// 将处理后的行写回文件。
fn write_lines(path: &Path, lines: &[String]) -> Result<(), PlanError> {
    let mut output = String::new();
    for line in lines {
        output.push_str(line);
        output.push('\n');
    }
    std::fs::write(path, &output)?;
    Ok(())
}

/// LLM 编辑：处理规则无法覆盖的复杂格式问题。
/// 支持 ROADMAP.md 和 TODO.md 两种格式，根据文件名切换 prompt。
fn edit_llm(
    content: &str,
    _scope: &str,
    settings: &quanttide_agent::Settings,
    path: &Path,
) -> Result<Option<Vec<Issue>>, PlanError> {
    use quanttide_agent::{llm::CompleteOptions, Message, LLM};

    let (_format_spec, system_role, prompt) = build_llm_prompt(content, path);

    let llm = LLM::new(
        &settings.llm_model,
        &settings.llm_base_url,
        &settings.llm_api_key,
    );
    let messages = vec![
        Message::new("system", system_role),
        Message::new("user", &prompt),
    ];
    let response = llm
        .complete(&messages, CompleteOptions::default())
        .map_err(|e| PlanError::Other(format!("LLM 调用失败: {}", e)))?;

    apply_llm_response(path, content, &response.content)
}

/// 组装 LLM prompt：根据文件名选择格式规范。
fn build_llm_prompt<'a>(content: &'a str, path: &Path) -> (&'static str, &'static str, String) {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ROADMAP.md");

    let (format_spec, system_role) = if file_name == "TODO.md" {
        (
            "TODO.md 格式规范：
a) 章节标题：## 标题（描述性 prose 标题，不含版本号）
b) 条目格式：- [ ] `文件路径` 操作描述：说明
c) 所有 TODO 条目必须引用文件路径（如 `src/main.rs`），缺少路径的条目请根据描述推断并补充路径",
            "你是 TODO.md 格式修复助手。确保每个条目引用文件路径，缺少路径时推断补充，不增删无关联条目。",
        )
    } else {
        (
            "ROADMAP.md 格式规范：
a) 版本标题：## [X.Y.Z]，可选后缀如 — 已发布
b) 分类标题：### Added / Changed / Fixed / Removed / Deprecated / Security
c) 条目格式：- [x] 内容 或 - [ ] 内容",
            "你是 ROADMAP.md 格式修复助手。只修格式，不增删条目内容。",
        )
    };

    let prompt = format!(
        "{}\n\n以下 {} 可能存在格式问题，请按规范修复格式（只修格式，不增删条目）：\n\n{}",
        format_spec, file_name, content
    );
    (format_spec, system_role, prompt)
}

/// 处理 LLM 响应：空或不变时跳过，否则写回文件。
fn apply_llm_response(
    path: &Path,
    original: &str,
    llm_output: &str,
) -> Result<Option<Vec<Issue>>, PlanError> {
    let fixed = llm_output.trim().to_string();
    if fixed.is_empty() || fixed == original {
        return Ok(None);
    }

    std::fs::write(path, &fixed)?;
    println!("  📋 LLM 格式修复已应用");

    Ok(Some(vec![Issue {
        line: 0,
        scope: String::new(),
        message: "LLM 格式修复完成".to_string(),
    }]))
}

/// LLM 语义审计：检查 ROADMAP 与 TODO 的一致性。
/// TODO 应是 ROADMAP 的待办子集，不应包含未规划的工作。
fn llm_audit_consistency(
    roadmap: &str,
    todo: &str,
    settings: &quanttide_agent::Settings,
) -> Result<Vec<String>, PlanError> {
    use quanttide_agent::{llm::CompleteOptions, Message, LLM};

    let prompt = build_consistency_prompt(roadmap, todo);
    let messages = vec![
        Message::new(
            "system",
            "你是一个严格的规划审计工具。只输出 JSON 数组，不要额外内容。",
        ),
        Message::new("user", &prompt),
    ];
    let options = CompleteOptions {
        response_format: Some(serde_json::json!({"type": "json_object"})),
        ..Default::default()
    };

    let llm = LLM::new(
        &settings.llm_model,
        &settings.llm_base_url,
        &settings.llm_api_key,
    );
    let resp = llm
        .complete(&messages, options)
        .map_err(|e| PlanError::Other(format!("LLM 调用失败: {}", e.0)))?;
    parse_audit_json(&resp.content)
}

/// 构建一致性审计的 LLM prompt。
fn build_consistency_prompt(roadmap: &str, todo: &str) -> String {
    format!(
        r#"你是一个规划一致性审计专家。ROADMAP.md 是完整规划，TODO.md 是待办子集。

要求：
1. TODO 中的每条待办是否能在 ROADMAP 中找到对应条目？
2. ROADMAP 中是否有高优先级条目应出现在 TODO 但缺失？
3. TODO 条目是否与 ROADMAP 的版本优先级相符？

只输出 JSON 数组，每项格式：{{"severity": "error|warn", "message": "..."}}
如果完全一致，输出空数组 []。

ROADMAP.md:
{}

TODO.md:
{}"#,
        roadmap, todo
    )
}

/// 解析 LLM 输出的 JSON 审计结果。
fn parse_audit_json(content: &str) -> Result<Vec<String>, PlanError> {
    let findings: Vec<serde_json::Value> = serde_json::from_str(content)
        .map_err(|e| PlanError::Other(format!("LLM 输出解析失败: {}", e)))?;
    let mut result = Vec::new();
    for f in &findings {
        if let (Some(severity), Some(msg)) = (
            f.get("severity").and_then(|s| s.as_str()),
            f.get("message").and_then(|s| s.as_str()),
        ) {
            result.push(format!("[{}] {}", severity, msg));
        }
    }
    Ok(result)
}

/// 从 TODO 条目行中提取反引号包裹的文件路径。
/// 支持 `:N` 行号后缀（如 `src/foo.rs:123`），自动剥离。
/// 只提取包含 `/` 或以常见扩展名结尾的 token（如 `src/main.rs`、`packages/foo`）。
fn extract_line_paths(line: &str) -> Vec<String> {
    line.split('`')
        .skip(1)
        .step_by(2)
        .filter_map(|s| {
            let s = s.trim();
            let path = s.split(':').next().unwrap_or(s);
            if (path.contains('/')
                || path.ends_with(".rs")
                || path.ends_with(".md")
                || path.ends_with(".toml"))
                && !path.starts_with('-')
                && !path.starts_with('[')
            {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// 格式审计：检查 ROADMAP.md 和 TODO.md 的格式合规性。
fn audit_format(roadmap_path: &Path, todo_path: &Path) -> Result<bool, PlanError> {
    let mut all_ok = true;
    for (path, label) in &[(roadmap_path, "ROADMAP.md"), (todo_path, "TODO.md")] {
        if !path.exists() { continue; }
        let issues = edit_roadmap(path, "(root)")?;
        if !issues.is_empty() {
            all_ok = false;
            println!("  ❌ {} 格式问题: {} 处", label, issues.len());
            for i in &issues { println!("     L{}: {}", i.line, i.message); }
        } else {
            println!("  ✅ {} 格式规范", label);
        }
    }
    Ok(all_ok)
}

/// 路径与粒度检查：扫描 TODO.md 条目中的路径引用是否有效。
fn audit_todo_paths(todo_path: &Path, dir: &Path) -> Result<bool, PlanError> {
    if !todo_path.exists() { return Ok(true); }
    let content = std::fs::read_to_string(todo_path)?;
    let mut path_missing_count = 0u32;
    let mut no_path_count = 0u32;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("- [") { continue; }
        let paths = extract_line_paths(trimmed);
        if paths.is_empty() {
            no_path_count += 1;
            continue;
        }
        for p in &paths {
            if !dir.join(p).exists() {
                path_missing_count += 1;
                println!("  ⚠ 路径不存在: {}", p);
            }
        }
    }
    if path_missing_count > 0 || no_path_count > 0 {
        if no_path_count > 0 { println!("  ⚠ {} 条 TODO 条目未引用文件路径", no_path_count); }
        Ok(false)
    } else {
        println!("  ✅ TODO.md 路径引用均有效");
        Ok(true)
    }
}

/// 孤儿 ROADMAP 条目检查：检测无路径引用的条目。
fn audit_orphan_roadmap(roadmap_path: &Path, todo_path: &Path) -> Result<bool, PlanError> {
    if let Ok(content) = std::fs::read_to_string(roadmap_path) {
        let has_orphan = content.lines().any(|l| {
            let t = l.trim();
            t.starts_with("- [ ]") && !t.contains('`')
        });
        if has_orphan {
            println!("  ⚠ ROADMAP.md 存在无路径引用的条目");
            return Ok(false);
        }
    } else if todo_path.exists() {
        println!("  ⚠ TODO.md 存在但无 ROADMAP.md（建议从 ROADMAP 派生）");
        return Ok(false);
    }
    Ok(true)
}

/// 审计规划：ROADMAP 是完整规划，TODO 是待办。
/// 检查格式合规、条目一致性、路径存在性、粒度。
pub fn plan_audit(repo_path: &Path) -> Result<(), PlanError> {
    let dir = resolve_plan_dir(repo_path, None);
    let roadmap_path = dir.join("ROADMAP.md");
    let todo_path = dir.join("TODO.md");

    println!("规划审计\n{}", "-".repeat(50));

    if !roadmap_path.exists() && !todo_path.exists() {
        println!("  未找到 ROADMAP.md 或 TODO.md");
        return Ok(());
    }

    let mut all_ok = true;

    all_ok &= audit_format(&roadmap_path, &todo_path)?;
    all_ok &= audit_todo_paths(&todo_path, &dir)?;
    all_ok &= audit_orphan_roadmap(&roadmap_path, &todo_path)?;

    // ── 4. LLM 语义审计 ─────────────────────────────────────────
    if roadmap_path.exists() && todo_path.exists() {
        let roadmap_content = std::fs::read_to_string(&roadmap_path)?;
        let todo_content = std::fs::read_to_string(&todo_path)?;
        let settings = quanttide_agent::Settings::from_env();
        if !settings.llm_api_key.is_empty() && cfg!(not(test)) {
            match llm_audit_consistency(&roadmap_content, &todo_content, &settings) {
                Ok(audit_result) => {
                    if audit_result.is_empty() {
                        println!("  ✅ LLM 语义审计: 一致");
                    } else {
                        all_ok = false;
                        println!("  ❌ LLM 语义审计:");
                        for line in &audit_result {
                            println!("     • {}", line);
                        }
                    }
                }
                Err(e) => println!("  ⚠ LLM 语义审计失败: {}", e),
            }
        }
    }

    println!("\n{}", "-".repeat(50));
    if all_ok {
        println!("  ✅ 审计通过");
    } else {
        println!("  ⚠ 存在待修复问题");
    }
    if all_ok {
        Ok(())
    } else {
        Err(PlanError::Other("审计未通过".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_roadmap(content: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(d.path().join("ROADMAP.md")).unwrap();
        write!(f, "{}", content).unwrap();
        d
    }

    fn read_roadmap(d: &Path) -> String {
        std::fs::read_to_string(d.join("ROADMAP.md")).unwrap_or_default()
    }

    // ── parse_roadmap ────────────────────────────────────────────

    #[test]
    fn test_parse_empty() {
        let d = write_roadmap("");
        let result = parse_roadmap(&d.path().join("ROADMAP.md"));
        assert!(result.is_err(), "空文件应解析失败");
    }

    #[test]
    fn test_parse_single_version() {
        let d = write_roadmap(
            "# ROADMAP\n\
             ## [0.1.0]\n\
             \n\
             ### Added\n\
             - [x] feature a\n\
             - [ ] feature b\n\
             ### Fixed\n\
             - [x] bug c\n",
        );
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].version, "0.1.0");
        assert_eq!(v[0].done, 2);
        assert_eq!(v[0].total, 3);
    }

    #[test]
    fn test_parse_multi_version() {
        let d = write_roadmap(
            "# ROADMAP\n\
             ## [0.2.0]\n\
             - [x] done\n\
             - [ ] todo\n\
             \n\
             ## [0.1.0]\n\
             - [x] a\n\
             - [x] b\n",
        );
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].version, "0.2.0");
        assert_eq!(v[0].done, 1);
        assert_eq!(v[0].total, 2);
        assert_eq!(v[1].version, "0.1.0");
        assert_eq!(v[1].done, 2);
        assert_eq!(v[1].total, 2);
    }

    #[test]
    fn test_parse_v_prefix() {
        let d = write_roadmap("# ROADMAP\n## [v0.1.0]\n- [x] item\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v[0].version, "0.1.0");
    }

    #[test]
    fn test_parse_no_checkboxes() {
        let d = write_roadmap("# ROADMAP\n## [0.1.0]\n\njust text\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].done, 0);
        assert_eq!(v[0].total, 0);
    }

    #[test]
    fn test_parse_version_with_suffix() {
        let d = write_roadmap("# ROADMAP\n## [0.1.0] — 已发布\n- [x] done\n- [ ] todo\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].version, "0.1.0");
        assert_eq!(v[0].done, 1);
        assert_eq!(v[0].total, 2);
    }

    #[test]
    fn test_clean_version_with_suffix() {
        // 后缀版本头应被识别并可级联清理
        let d = write_roadmap("# ROADMAP\n## [0.1.0] — 已发布\n- [x] done\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        assert!(!content.contains("0.1.0"), "空版本应被清理");
    }

    #[test]
    fn test_parse_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let result = parse_roadmap(&d.path().join("NONEXISTENT.md"));
        assert!(result.is_err());
    }

    // ── resolve_roadmap_path ────────────────────────────────────

    #[test]
    fn test_resolve_path_with_contract_scope() {
        let d = tempfile::tempdir().unwrap();
        // 创建 scope 契约
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  cli:\n    dir: src/cli\n    language: rust\n",
        )
        .unwrap();
        let path = resolve_roadmap_path(d.path(), Some("cli"));
        assert!(path.to_string_lossy().ends_with("src/cli/ROADMAP.md"));
    }

    #[test]
    fn test_resolve_path_fallback_to_name() {
        let d = tempfile::tempdir().unwrap();
        let path = resolve_roadmap_path(d.path(), Some("custom"));
        // scope 不在契约中 → 回退为子目录名
        assert!(path.to_string_lossy().ends_with("custom/ROADMAP.md"));
    }

    #[test]
    fn test_resolve_path_no_scope_no_contract() {
        let d = tempfile::tempdir().unwrap();
        let path = resolve_roadmap_path(d.path(), None);
        // 无 scope + 无契约 → repo 根目录
        assert_eq!(path, d.path().join("ROADMAP.md"));
    }

    // ── clean_roadmap ───────────────────────────────────────────

    #[test]
    fn test_clean_removes_done_items() {
        let d = write_roadmap(
            "## [0.1.0]\n\
             ### Added\n\
             - [x] done item\n\
             - [ ] todo item\n\
             ### Fixed\n\
             - [x] fixed bug\n",
        );
        let removed = clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert!(removed > 0);
        let content = read_roadmap(d.path());
        assert!(!content.contains("done item"));
        assert!(!content.contains("fixed bug"));
        assert!(content.contains("todo item"));
    }

    #[test]
    fn test_clean_empty_file() {
        let d = write_roadmap("");
        let removed = clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_clean_all_done_empties_file() {
        // 所有条目都是 done → 清理后只剩空文件
        let d = write_roadmap("## [0.1.0]\n### Added\n- [x] done\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        assert!(content.is_empty());
    }

    #[test]
    fn test_clean_no_done_items_no_change() {
        let d = write_roadmap("## [0.1.0]\n- [ ] todo\n");
        let removed = clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_clean_cascade_does_not_delete_adjacent_version() {
        // Issue #5-4: [0.5.0] 全 done 被清后，[0.6.0] 不应被连带删除
        let content = "## [0.6.0]\n\
- [ ] 修复 bug\n\
\n\
## [0.5.0]\n\
- [x] 已删除 legacy\n";
        let d = write_roadmap(content);
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let result = read_roadmap(d.path());
        assert!(result.contains("0.6.0"), "[0.6.0] 不应被删除: {}", result);
        assert!(!result.contains("0.5.0"), "[0.5.0] 应被删除: {}", result);
        assert!(result.contains("修复 bug"), "内容应保留: {}", result);
    }

    #[test]
    fn test_clean_trailing_newlines_removed() {
        // 末尾多余空行应被清理
        let d = write_roadmap("## [0.1.0]\n- [ ] todo\n\n\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        assert_eq!(content.trim_end().lines().count(), 2); // 版本标题 + 条目
    }

    #[test]
    fn test_clean_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let nonexistent = d.path().join("NONEXISTENT.md");
        let result = clean_roadmap(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_clean_suffix_version_all_done_cascade() {
        let d = write_roadmap("## [0.2.0]\n\n- [ ] 待办\n\n## [0.1.0] — 已发布\n\n- [x] 旧功能\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        // 0.1.0 版本应被删除（全部 done 且带后缀）
        assert!(!content.contains("0.1.0"), "0.1.0 版本应被删除");
        // 0.2.0 版本应保留
        assert!(content.contains("0.2.0"), "0.2.0 版本应保留");
        // 待办内容应保留
        assert!(content.contains("待办"), "待办内容应保留");
    }

    // ── edit_roadmap ────────────────────────────────────────

    #[test]
    fn test_edit_fixes_v_prefix() {
        let d = write_roadmap("## [v0.1.0]\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(issues.iter().any(|f| f.message.contains("v 前缀")));
        let content = read_roadmap(d.path());
        assert!(!content.contains("## [v"));
    }

    #[test]
    fn test_edit_fixes_category_case() {
        let d = write_roadmap("## [0.1.0]\n### added\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(issues.iter().any(|f| f.message.contains("大小写")));
        let content = read_roadmap(d.path());
        assert!(content.contains("### Added"));
    }

    #[test]
    fn test_edit_clean_file_no_issues() {
        let d = write_roadmap("## [0.1.0]\n### Added\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_edit_modifies_file() {
        let d = write_roadmap("## [v0.1.0]\n### ADDED\n-  [x] bad\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(!issues.is_empty());
        let content = read_roadmap(d.path());
        assert!(content.contains("## [0.1.0]"));
        assert!(content.contains("### Added"));
        assert!(content.contains("- [x] bad"));
    }

    #[test]
    fn test_edit_detects_nonstandard_header() {
        let d = write_roadmap("## 现状 (Current)\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(
            issues.iter().any(|i| i.message.contains("非标准版本头")),
            "应检测到非标准版本头: {:?}",
            issues
        );
    }

    #[test]
    fn test_edit_detects_nonstandard_category() {
        let d = write_roadmap("## [0.1.0]\n### 0.1 fix bug\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(
            issues.iter().any(|i| i.message.contains("非标准分类")),
            "应检测到非标准分类: {:?}",
            issues
        );
    }

    #[test]
    fn test_edit_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let nonexistent = d.path().join("NONEXISTENT.md");
        let result = edit_roadmap(&nonexistent, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_mixed_format() {
        let d = write_roadmap("## [0.1.0]\n\n- [ ] 标准条目\n\n## 杂项 (Misc)\n\n- [ ] 非标准\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(
            issues.iter().any(|i| i.message.contains("非标准版本头")),
            "应检测到非标准版本头: {:?}",
            issues
        );
    }

    // ── print_status_to ─────────────────────────────────────────

    #[test]
    fn test_print_status_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("未创建规划文件"));
    }

    #[test]
    fn test_print_status_empty_roadmap() {
        let d = write_roadmap("");
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("未找到标准规划条目"));
    }

    #[test]
    fn test_print_status_unknown_headers_warns() {
        // 非标准 ## 头应触发 warning
        let d = write_roadmap("# ROADMAP\n## 现状 (Current)\n- [ ] item\n");
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(
            output.contains("plan edit"),
            "应提示运行 plan edit: {}",
            output
        );
    }

    #[test]
    fn test_print_status_to_with_scope() {
        // scope "test" 不在契约中 → 回退到 test/ROADMAP.md
        let d = tempfile::tempdir().unwrap();
        let scope_dir = d.path().join("test");
        std::fs::create_dir_all(&scope_dir).unwrap();
        std::fs::write(
            scope_dir.join("ROADMAP.md"),
            "# ROADMAP\n## [0.1.0]\n- [x] done\n- [ ] todo\n",
        )
        .unwrap();
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), Some("test")).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("test"), "应显示 scope 名称");
        assert!(out.contains("0.1.0"), "应显示版本号");
    }

    #[test]
    fn test_print_status_with_data() {
        let d =
            write_roadmap("# ROADMAP\n## [0.2.0]\n- [x] done\n- [ ] todo\n\n## [0.1.0]\n- [x] a\n- [x] b\n");
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("(auto)"));
        assert!(output.contains("0.2.0"));
        assert!(output.contains("0.1.0"));
        assert!(output.contains("3/4"));
        assert!(output.contains("总计"));
    }

    // ── extract_line_paths ───────────────────────────────────────

    #[test]
    fn test_extract_line_paths_simple() {
        let paths = extract_line_paths("- [ ] `src/main.rs` `run_plan_clean`：重构");
        assert_eq!(paths, vec!["src/main.rs"]);
    }

    #[test]
    fn test_extract_line_paths_multiple() {
        let paths = extract_line_paths("- [ ] `src/plan.rs` `plan_audit`：新增路径检查");
        assert_eq!(paths, vec!["src/plan.rs"]);
    }

    #[test]
    fn test_extract_line_paths_no_path_returns_empty() {
        let paths = extract_line_paths("- [ ] 修复登录页面样式");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_extract_line_paths_skip_non_path_backtick() {
        let paths = extract_line_paths("- [ ] `clean` 命令支持 `--all` 参数");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_extract_line_paths_strips_line_number() {
        let paths = extract_line_paths("- [ ] `src/foo.rs:123` 修复 bug");
        assert_eq!(paths, vec!["src/foo.rs"]);
    }

    #[test]
    fn test_extract_line_paths_with_colon_no_number() {
        let paths = extract_line_paths("- [ ] `docs/README.md` 更新文档");
        assert_eq!(paths, vec!["docs/README.md"]);
    }

    // ── plan_audit path checks ───────────────────────────────────

    fn write_todo(content: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("TODO.md"), content).unwrap();
        d
    }

    #[test]
    fn test_audit_path_missing() {
        let d = write_todo("- [ ] `nonexistent/module.rs`：待实现\n");
        // 创建空 ROADMAP.md 避免提前退出
        let _ = std::fs::write(d.path().join("ROADMAP.md"), "");
        let result = plan_audit(d.path());
        assert!(result.is_err(), "路径不存在应使审计失败");
    }

    #[test]
    fn test_audit_granularity_warn() {
        let d = write_todo("- [ ] 缺少文件路径的条目\n");
        let _ = std::fs::write(d.path().join("ROADMAP.md"), "");
        let result = plan_audit(d.path());
        assert!(result.is_err(), "无路径条目应使审计失败");
    }

    #[test]
    fn test_audit_path_exists() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/main.rs"), "").unwrap();
        std::fs::write(
            d.path().join("TODO.md"),
            "- [ ] `src/main.rs` 实现功能\n",
        )
        .unwrap();
        let _ = std::fs::write(d.path().join("ROADMAP.md"), "");
        let result = plan_audit(d.path());
        assert!(result.is_ok(), "路径存在应通过审计: {:?}", result);
    }
}
