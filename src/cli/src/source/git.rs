use std::path::Path;
use std::process::Command;

/// 在 `cwd` 下执行 git 命令，成功时返回 stdout（去尾空白），失败返回错误描述。
pub fn git(args: &[&str], cwd: &Path) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git 无法执行: {}", e))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "git 命令失败".into()
        } else {
            stderr
        })
    }
}

/// 执行 git 命令，返回 `Option<bool>` 表示成功与否（不关心 stdout 内容）。
pub fn git_check(args: &[&str], cwd: &Path) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 检查路径是否为 git 仓库（存在 `.git` 目录或文件）。
pub fn is_git_repo(path: &Path) -> bool {
    let git_dir = path.join(".git");
    git_dir.is_dir() || git_dir.is_file()
}

/// 统计 `tag..HEAD` 之间的提交数。
///
/// `path_filter` 可选，限制路径范围。返回 `Some(n)` 表示成功。
pub fn rev_list_count(repo_path: &Path, tag: &str, path_filter: Option<&str>) -> Option<usize> {
    let range = format!("{}..HEAD", tag);
    let mut args = vec!["rev-list", "--count", &range];
    if let Some(filter) = path_filter {
        if !filter.is_empty() && filter != "." {
            args.push("--");
            args.push(filter);
        }
    }
    let out = Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        s.parse::<usize>().ok()
    } else {
        None
    }
}

/// 检查工作区是否有未提交的变更。
pub fn is_working_tree_dirty(repo_path: &Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// 检查指定 ref 是否存在（如 tag、branch）。
pub fn ref_exists(repo_path: &Path, refname: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", refname])
        .current_dir(repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 解析 git log --oneline 输出为提交消息列表。
pub fn parse_commit_messages(log_output: &str) -> Vec<String> {
    log_output
        .lines()
        .map(|l| {
            if l.len() > 8 {
                l[7..].trim().to_string()
            } else {
                l.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// 获取自最新 tag 以来的变更文件列表。
pub fn get_changed_paths_since_last_tag(root: &Path) -> Vec<String> {
    let tags = crate::source::tag::collect_tags_with_scope(root);
    let latest_tag = tags
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .find_map(|(_, v)| v.first())
        .or_else(|| tags.get("(root)").and_then(|v| v.first()));

    let range = match latest_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => return vec![],
    };

    let output = git(&["diff", "--name-only", &range], root).unwrap_or_default();
    output.lines().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_repo_dir() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join(".git")).unwrap();
        assert!(is_git_repo(d.path()));
    }

    #[test]
    fn test_is_git_repo_file() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".git"), "gitdir: ../.git/modules/foo").unwrap();
        assert!(is_git_repo(d.path()));
    }

    #[test]
    fn test_is_git_repo_false() {
        let d = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(d.path()));
    }

    #[test]
    fn test_is_working_tree_dirty_in_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        let dirty = is_working_tree_dirty(d.path());
        assert!(!dirty);
    }

    #[test]
    fn test_parse_commit_messages_typical() {
        let msgs = parse_commit_messages("abc1234 feat: add foo\ndef5678 fix: bar\n");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0], "feat: add foo");
        assert_eq!(msgs[1], "fix: bar");
    }

    #[test]
    fn test_parse_commit_messages_empty() {
        assert!(parse_commit_messages("").is_empty());
    }

    #[test]
    fn test_parse_commit_messages_short_line() {
        let msgs = parse_commit_messages("abc1234\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], "abc1234");
    }
}
