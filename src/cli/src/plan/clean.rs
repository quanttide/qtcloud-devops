use super::PlanError;
use crate::source::roadmap::{is_version_line, CATEGORIES};
use std::path::Path;

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

    remove_empty_categories(&mut lines);
    remove_empty_versions(&mut lines);
    clean_trailing_blanks(&mut lines);

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

/// 删除空的分类标题（`### Added` 等，其后无实际内容）。
fn remove_empty_categories(lines: &mut Vec<&str>) {
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
}

/// 删除空的版本标题（`## [X.Y.Z]`，其后无内容）。
fn remove_empty_versions(lines: &mut Vec<&str>) {
    let mut i = 0;
    while i < lines.len() {
        if is_version_line(lines[i]).is_some() {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j >= lines.len() || is_version_line(lines[j]).is_some() {
                lines.remove(i);
                continue;
            }
        }
        i += 1;
    }
    if let Some(last) = lines.last() {
        if is_version_line(last).is_some() {
            lines.pop();
        }
    }
}

/// 清理尾部空行。
fn clean_trailing_blanks(lines: &mut Vec<&str>) {
    while let Some(last) = lines.last() {
        if last.trim().is_empty() {
            lines.pop();
        } else {
            break;
        }
    }
}
