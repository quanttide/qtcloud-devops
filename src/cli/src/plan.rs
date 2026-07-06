/// plan 命令：ROADMAP.md 规划管理。
///
/// 对应 `data/roadmap/platform/plan-command.md`。
///
/// 三个子命令：
/// - `status` — 查看 scope 规划进度
/// - `clean`  — 删除已完成条目
/// - `doctor` — 修复格式问题（规则修复 + LLM 修复）
use std::path::{Path, PathBuf};

use quanttide_devops::source::roadmap::RoadmapVersion;

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

/// 解析 ROADMAP.md，返回各版本进度列表。
pub fn parse_roadmap(path: &Path) -> Result<Vec<RoadmapVersion>, PlanError> {
    let content = std::fs::read_to_string(path)?;
    parse_roadmap_str(&content)
}

/// 解析 ROADMAP.md 字符串，返回各版本进度列表。
pub fn parse_roadmap_str(s: &str) -> Result<Vec<RoadmapVersion>, PlanError> {
    let mut versions: Vec<RoadmapVersion> = Vec::new();
    let lines: Vec<&str> = s.lines().collect();
    let mut i = 0;
    let mut current_version: Option<String> = None;
    let mut done = 0usize;
    let mut total = 0usize;
    let mut categories: Vec<(String, Vec<quanttide_devops::source::roadmap::RoadmapChecklistItem>)> = Vec::new();
    let mut current_cat: Option<String> = None;
    let mut cat_items: Vec<quanttide_devops::source::roadmap::RoadmapChecklistItem> = Vec::new();

    while i < lines.len() {
        let t = lines[i].trim();
        if let Some(ver) = is_version_line(t) {
            if let Some(v) = current_version.take() {
                if let Some(cat) = current_cat.take() { categories.push((cat, cat_items.clone())); cat_items.clear(); }
                versions.push(RoadmapVersion { version: v, status: String::new(), done, total, categories: std::mem::take(&mut categories) });
            }
            done = 0; total = 0;
            current_version = Some(ver);
        } else if t.starts_with("### ") && current_version.is_some() {
            if let Some(cat) = current_cat.take() { categories.push((cat, std::mem::take(&mut cat_items))); }
            current_cat = Some(t[4..].trim().to_string());
        } else if current_version.is_some() {
            if t.starts_with("- [x]") || t.starts_with("- [X]") {
                total += 1; done += 1;
                cat_items.push(quanttide_devops::source::roadmap::RoadmapChecklistItem { description: t[5..].trim().to_string(), completed: true });
            } else if t.starts_with("- [ ]") {
                total += 1;
                cat_items.push(quanttide_devops::source::roadmap::RoadmapChecklistItem { description: t[5..].trim().to_string(), completed: false });
            }
        }
        i += 1;
    }
    if let Some(cat) = current_cat.take() { categories.push((cat, cat_items)); }
    if let Some(ver) = current_version {
        versions.push(RoadmapVersion { version: ver, status: String::new(), done, total, categories });
    }
    Ok(versions)
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
        writeln!(writer, "  未创建规划文件: {}", plan_dir.join("ROADMAP.md").display()).ok();
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
    let versions = parse_roadmap(path)?;
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
                    && !CATEGORIES.iter().any(|c| c.to_lowercase() == t.to_lowercase()))
        });
        if has_unknown_headers {
            let settings = quanttide_agent::Settings::from_env();
        if !settings.llm_api_key.is_empty() && !settings.llm_base_url.is_empty() && !settings.llm_model.is_empty() && cfg!(not(test)) {
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
            writeln!(writer, "  ⚠ 文件含有非标准格式的标题，运行 `plan edit` 查看详情").ok();
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
        writeln!(writer, "  [{:<8}] {:>2}/{:>2} 完成 ({:.0}%)", v.version, v.done, v.total, rate).ok();
        total_done += v.done;
        total_all += v.total;
    }

    let overall = if total_all > 0 {
        total_done as f64 / total_all as f64 * 100.0
    } else {
        0.0
    };
    writeln!(writer,  "  {}", "-".repeat(40)).ok();
    writeln!(writer, "  总计:  {}/{} 完成 ({:.0}%)", total_done, total_all, overall).ok();
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
];

fn is_done_item(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("- [x]") || t.starts_with("- [X]")
}

fn is_category_header(line: &str) -> bool {
    let t = line.trim();
    CATEGORIES
        .iter()
        .any(|c| t == *c || t.eq_ignore_ascii_case(c))
}

fn is_version_header(line: &str) -> bool {
    is_version_line(line).is_some()
}

/// 删除 ROADMAP.md 中所有已完成条目。
///
/// 只删 `- [x]` 行，级联清理空分类和空版本标题。
pub fn clean_roadmap(path: &Path) -> Result<usize, PlanError> {
    let content = std::fs::read_to_string(path)?;
    let original_len = content.len();

    let mut lines: Vec<&str> = content.lines().collect();

    // 第一遍：删除 done item 行
    lines.retain(|l| !is_done_item(l));

    // 第二遍：删除空的分类标题（跳过空行看后面是否真有内容）
    let mut i = 0;
    while i < lines.len() {
        if is_category_header(lines[i]) {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j >= lines.len() || is_category_header(lines[j]) || is_version_header(lines[j]) {
                lines.remove(i);
                continue;
            }
        }
        i += 1;
    }

    // 第三遍：删除空的版本标题（跳过空行看后面是否真有内容）
    let mut i = 0;
    while i < lines.len() {
        if is_version_header(lines[i]) {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j >= lines.len() || is_version_header(lines[j]) {
                // 后面是文件尾或另一个版本头 → 此版本为空
                lines.remove(i);
                continue;
            }
            // 后面有内容（checkbox、分类等）→ 保留
        }
        i += 1;
    }
    if let Some(last) = lines.last() {
        if is_version_header(last) {
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
        return Ok(original_len);
    }

    let mut output = String::new();
    for line in &lines {
        output.push_str(line);
        output.push('\n');
    }
    std::fs::write(path, &output)?;
    Ok(original_len.saturating_sub(output.len()))
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

        // 0a. 检测非标准 ## 头（如 ## P0 — 阻塞）
        if trimmed.starts_with("## ") && !is_version_line(trimmed).is_some() {
            issues.push(Issue {
                line: line_num,
                scope: scope.to_string(),
                message: format!("非标准版本头（应为 ## [X.Y.Z]）: {}", trimmed),
            });
            new_lines.push(raw_line.to_string());
            continue;
        }

        // 0b. 检测非标准 ### 分类（如 ### 0.1 xxx）

        // 0b. 检测非标准 ### 分类（如 ### 0.1 xxx）
        if trimmed.starts_with("### ")
            && !CATEGORIES
                .iter()
                .any(|c| c.to_lowercase() == trimmed.to_lowercase())
        {
            issues.push(Issue {
                line: line_num,
                scope: scope.to_string(),
                message: format!("非标准分类标题: {}", trimmed),
            });
            new_lines.push(raw_line.to_string());
            continue;
        }

        // 1. 版本标题：去掉 v 前缀
        if let Some(ver) = is_version_line(trimmed) {
            let raw_ver = trimmed
                .trim_start_matches("## [")
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            if raw_ver.starts_with('v') {
                issues.push(Issue {
                    line: line_num,
                    scope: scope.to_string(),
                    message: format!("修复 v 前缀: {} → {}", raw_ver, ver),
                });
                let suffix = trimmed.split(']').nth(1).unwrap_or("");
                new_lines.push(format!("## [{}]{}", ver, suffix));
                continue;
            }
            new_lines.push(raw_line.to_string());
            continue;
        }

        // 2. 分类标题：标准化大小写
        if trimmed.starts_with("### ") {
            let lowered = trimmed.to_lowercase();
            if let Some(standard) = CATEGORIES.iter().find(|c| c.to_lowercase() == lowered) {
                if trimmed != *standard {
                    issues.push(Issue {
                        line: line_num,
                        scope: scope.to_string(),
                        message: format!("修复大小写: {} → {}", trimmed, standard),
                    });
                    let indent = &raw_line[..raw_line.len() - raw_line.trim_start().len()];
                    new_lines.push(format!("{}{}", indent, standard));
                    continue;
                }
            }
            new_lines.push(raw_line.to_string());
            continue;
        }

        // 3. checkbox：修复异常格式
        let has_any_box =
            trimmed.contains("[x]") || trimmed.contains("[X]") || trimmed.contains("[ ]");
        let is_standard = trimmed.starts_with("- [x] ")
            || trimmed.starts_with("- [X] ")
            || trimmed.starts_with("- [ ] ");
        if has_any_box && !is_standard {
            let content_start = trimmed.find(']').map(|p| p + 1).unwrap_or(trimmed.len());
            let item_content = trimmed[content_start..].trim();
            let is_done = trimmed.contains("[x]") || trimmed.contains("[X]");
            let prefix = if is_done { "- [x]" } else { "- [ ]" };
            issues.push(Issue {
                line: line_num,
                scope: scope.to_string(),
                message: format!(
                    "修复 checkbox 格式: {} → {} {}",
                    trimmed, prefix, item_content
                ),
            });
            new_lines.push(format!("{} {}", prefix, item_content));
            continue;
        }

        new_lines.push(raw_line.to_string());
    }

    if !issues.is_empty() {
        let mut output = String::new();
        for line in &new_lines {
            output.push_str(line);
            output.push('\n');
        }
        std::fs::write(path, &output)?;
    }

    Ok(issues)
}

/// LLM 编辑：处理规则无法覆盖的复杂格式问题。
fn edit_llm(
    content: &str,
    _scope: &str,
    settings: &quanttide_agent::Settings,
    path: &Path,
) -> Result<Option<Vec<Issue>>, PlanError> {
    use quanttide_agent::{llm::CompleteOptions, Message, LLM};

    let format_spec = "ROADMAP.md 格式规范：
a) 版本标题：## [X.Y.Z]，可选后缀如 — 已发布
b) 分类标题：### Added / Changed / Fixed / Removed / Deprecated / Security
c) 条目格式：- [x] 内容 或 - [ ] 内容
";
    let prompt = format!(
        "{}\n\n以下 ROADMAP.md 可能存在格式问题，请按规范修复格式（只修格式，不增删条目）：\n\n{}",
        format_spec, content
    );

    let llm = LLM::new(
        &settings.llm_model,
        &settings.llm_base_url,
        &settings.llm_api_key,
    );
    let messages = vec![
        Message::new(
            "system",
            "你是 ROADMAP.md 格式修复助手。只修格式，不增删条目内容。",
        ),
        Message::new("user", &prompt),
    ];
    let response = llm
        .complete(&messages, CompleteOptions::default())
        .map_err(|e| PlanError::Other(format!("LLM 调用失败: {}", e)))?;

    let fixed = response.content.trim().to_string();
    if fixed.is_empty() || fixed == content {
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
    let llm = LLM::new(&settings.llm_model, &settings.llm_base_url, &settings.llm_api_key);
    let prompt = format!(
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
    );
    let messages = vec![
        Message::new("system", "你是一个严格的规划审计工具。只输出 JSON 数组，不要额外内容。"),
        Message::new("user", &prompt),
    ];
    let options = CompleteOptions {
        response_format: Some(serde_json::json!({"type": "json_object"})),
        ..Default::default()
    };
    let resp = llm.complete(&messages, options)
        .map_err(|e| PlanError::Other(format!("LLM 调用失败: {}", e.0)))?;
    let findings: Vec<serde_json::Value> = serde_json::from_str(&resp.content)
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

/// 审计规划：ROADMAP 是完整规划，TODO 是待办。
/// 检查格式合规、条目一致性（TODO 不应包含 ROADMAP 未规划的工作）。
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

    // ── 1. 格式审计 ───────────────────────────────────────────────
    if roadmap_path.exists() {
        let issues = edit_roadmap(&roadmap_path, "(root)")?;
        if !issues.is_empty() {
            all_ok = false;
            println!("  ❌ ROADMAP.md 格式问题: {} 处", issues.len());
            for i in &issues {
                println!("     L{}: {}", i.line, i.message);
            }
        } else {
            println!("  ✅ ROADMAP.md 格式规范");
        }
    }

    if todo_path.exists() {
        let issues = edit_roadmap(&todo_path, "(root)")?;
        if !issues.is_empty() {
            all_ok = false;
            println!("  ❌ TODO.md 格式问题: {} 处", issues.len());
            for i in &issues {
                println!("     L{}: {}", i.line, i.message);
            }
        } else {
            println!("  ✅ TODO.md 格式规范");
        }
    }

    // ── 2. LLM 语义审计 ─────────────────────────────────────────
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
    } else if todo_path.exists() && !roadmap_path.exists() {
        println!("  ⚠ TODO.md 存在但无 ROADMAP.md（建议从 ROADMAP 派生）");
        all_ok = false;
    }

    println!("\n{}", "-".repeat(50));
    if all_ok {
        println!("  ✅ 审计通过");
    } else {
        println!("  ⚠ 存在待修复问题");
    }
    if all_ok { Ok(()) } else { Err(PlanError::Other("审计未通过".into())) }
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
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert!(v.is_empty());
    }

    #[test]
    fn test_parse_single_version() {
        let d = write_roadmap(
            "## [0.1.0]\n\
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
            "## [0.2.0]\n\
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
        let d = write_roadmap("## [v0.1.0]\n- [x] item\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v[0].version, "0.1.0");
    }

    #[test]
    fn test_parse_no_checkboxes() {
        let d = write_roadmap("## [0.1.0]\n\njust text\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].done, 0);
        assert_eq!(v[0].total, 0);
    }

    #[test]
    fn test_parse_version_with_suffix() {
        // `## [0.1.0] — 已发布` 应被正确识别
        let d = write_roadmap("## [0.1.0] — 已发布\n- [x] done\n- [ ] todo\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].version, "0.1.0");
        assert_eq!(v[0].done, 1);
        assert_eq!(v[0].total, 2);
    }

    #[test]
    fn test_clean_version_with_suffix() {
        // 后缀版本头应被识别并可级联清理
        let d = write_roadmap("## [0.1.0] — 已发布\n- [x] done\n");
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
        let d = write_roadmap("## 现状 (Current)\n- [ ] item\n");
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
            "## [0.1.0]\n- [x] done\n- [ ] todo\n",
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
            write_roadmap("## [0.2.0]\n- [x] done\n- [ ] todo\n\n## [0.1.0]\n- [x] a\n- [x] b\n");
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("(auto)"));
        assert!(output.contains("0.2.0"));
        assert!(output.contains("0.1.0"));
        assert!(output.contains("3/4"));
        assert!(output.contains("总计"));
    }
}
