use super::{Issue, PlanError};
use crate::source::roadmap::apply_rules_to_line;
use std::path::Path;

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
pub(crate) fn edit_llm(
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
pub(crate) fn apply_llm_response(
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

pub fn doctor_file(path: &Path, scope: &str) -> Result<Vec<Issue>, PlanError> {
    edit_roadmap(path, scope)
}
