use std::path::Path;

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
    let out = std::process::Command::new("git")
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

#[cfg(test)]
mod tests {
    use super::*;

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
