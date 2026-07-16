use super::PlanError;
use crate::plan::{status::resolve_roadmap_dir, edit_roadmap};
use std::path::Path;


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
pub(crate) fn extract_line_paths(line: &str) -> Vec<String> {
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
    let dir = resolve_roadmap_dir(repo_path, None);
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

