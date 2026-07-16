use super::{PlanError, Issue};
use std::path::{Path, PathBuf};
use std::io::Write;
use quanttide_devops::source::roadmap::{Roadmap, RoadmapVersion};
use crate::source::roadmap::{apply_rules_to_line, is_version_line, CATEGORIES};
use crate::plan::doctor::edit_llm;

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
            // 相对化 current_dir，find_scope_by_path 用 scope.dir（相对路径）做 starts_with
            let relative = current_dir.strip_prefix(repo_path).unwrap_or(&current_dir);
            if let Some(s) = c.find_scope_by_path(relative) {
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
            let relative = current_dir.strip_prefix(repo_path).unwrap_or(&current_dir);
            if let Some(s) = c.find_scope_by_path(relative) {
                repo_path.join(&s.dir)
            } else {
                repo_path.to_path_buf()
            }
        }
    }
}

/// 修复 ROADMAP 或 TODO 文件的格式。
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
pub(crate) fn resolve_plan_dir(repo_path: &Path, scope: Option<&str>) -> PathBuf {
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
