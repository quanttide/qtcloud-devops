use super::PlanError;
use crate::plan::status::resolve_roadmap_dir;
use std::path::Path;

pub fn todo_from_audit(repo_path: &Path, json: &str, scope: Option<&str>) -> Result<(), PlanError> {
    use crate::code::AuditPlan;

    let plan: AuditPlan = serde_json::from_str(json)
        .map_err(|e| PlanError::Other(format!("JSON 解析失败: {}", e)))?;
    let todo_path = resolve_roadmap_dir(repo_path, scope).join("TODO.md");

    let mut content = if todo_path.exists() {
        std::fs::read_to_string(&todo_path)?
    } else {
        String::new()
    };

    // 构建新节内容
    let section_header = format!("### {}", plan.source_label);
    let mut new_section = format!("{}\n\n", section_header);
    for entry in &plan.entries {
        new_section.push_str(&format!("#### {}\n\n", entry.priority));
        for item in &entry.items {
            new_section.push_str(&format!("- [ ] `{}`: {}\n", item.file, item.detail));
        }
        new_section.push('\n');
    }

    // 替换现有节或追加
    if content.contains(&section_header) {
        let start = content.find(&section_header).unwrap();
        let end = content[start + 1..]
            .find("\n### ")
            .map(|p| start + 1 + p)
            .unwrap_or(content.len());
        content.replace_range(start..end, new_section.trim_end());
    } else {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&new_section);
    }

    std::fs::write(&todo_path, &content)?;
    println!("  ✓ 已更新 {}", todo_path.display());
    Ok(())
}

/// 从审计 JSON 更新 ROADMAP.md。
/// 按检查名聚合，不区分 >80 和 >40——最终目标一致。
pub fn roadmap_from_audit(
    repo_path: &Path,
    json: &str,
    scope: Option<&str>,
) -> Result<(), PlanError> {
    use crate::code::AuditPlan;

    let plan: AuditPlan = serde_json::from_str(json)
        .map_err(|e| PlanError::Other(format!("JSON 解析失败: {}", e)))?;
    let roadmap_path = resolve_roadmap_dir(repo_path, scope).join("ROADMAP.md");

    let mut content = if roadmap_path.exists() {
        std::fs::read_to_string(&roadmap_path)?
    } else {
        String::new()
    };

    // 按 check 名聚合计数
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &plan.entries {
        for item in &entry.items {
            *counts.entry(item.check.clone()).or_insert(0) += 1;
        }
    }

    // 构建 ### 节
    let mut new_section = String::from("### 代码质量重构\n\n");
    for (check, count) in &counts {
        let desc = check_to_roadmap_desc(check, *count);
        // 保留已有 [x] 状态
        let prefix = if content.contains(&format!("- [x] {}", desc)) {
            "- [x]"
        } else {
            "- [ ]"
        };
        new_section.push_str(&format!("{} {}\n", prefix, desc));
    }

    // 定位或创建版本头
    let version_header = format!("## [{}]", detect_roadmap_version(&content));
    if !content.contains(&version_header) {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("\n{}\n\n", version_header));
    }

    // 替换 `### 代码质量重构` 节
    let section_header = "### 代码质量重构";
    if content.contains(section_header) {
        let start = content.find(section_header).unwrap();
        let end = content[start + 1..]
            .find("\n## ")
            .map(|p| start + 1 + p)
            .unwrap_or(content.len());
        content.replace_range(start..end, new_section.trim_end());
    } else {
        // 在版本头后追加
        if let Some(pos) = content.find(&version_header) {
            let insert_at = pos + version_header.len();
            content.insert_str(insert_at, &format!("\n{}\n", new_section.trim_end()));
        } else {
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&new_section);
        }
    }

    std::fs::write(&roadmap_path, &content)?;
    println!("  ✓ 已更新 {}", roadmap_path.display());
    Ok(())
}

/// 从现有 ROADMAP.md 检测版本号，取第一个 `## [X.Y.Z]`。
fn detect_roadmap_version(content: &str) -> String {
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("## [") {
            if let Some(ver) = rest.split(']').next() {
                return ver.to_string();
            }
        }
    }
    "0.0.0".to_string()
}

/// 将检查名映射为 ROADMAP 条目描述。
fn check_to_roadmap_desc(check: &str, count: usize) -> String {
    match check {
        "Scope 目录" => "所有 scope 目录存在".to_string(),
        "TODO/FIXME 密度" => "TODO/FIXME 密度降至 5‰ 以下".to_string(),
        "函数长度" => format!("全部函数长度控制在 40 行以内（{} 处）", count),
        "API 文档覆盖率" => format!("全部 pub 函数包含 /// 文档注释（{} 处）", count),
        "结构复杂度" => format!("圈复杂度 ≤ 10 / 嵌套 ≤ 4 层（{} 处）", count),
        "导入数" => format!("导入数控制在 30 以内（{} 处）", count),
        "文件长度" | "超长文件（阈值 500 行）" => {
            format!("文件长度控制在 500 行以内（{} 处）", count)
        }
        "模块文档" => format!("模块文档覆盖率达到 100%（{} 处）", count),
        "语法检查" => "语法检查全部通过".to_string(),
        _ => format!("{}({} 处)", check, count),
    }
}
