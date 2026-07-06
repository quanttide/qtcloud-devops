use std::collections::HashMap;
use std::path::Path;

use crate::contract;

pub fn status(repo_path: &Path) {
    let mut stdout = std::io::stdout();
    status_to(&mut stdout, repo_path).ok();
}

pub fn status_to(writer: &mut impl std::io::Write, repo_path: &Path) -> std::io::Result<()> {
    let scopes_map = load_scopes_map(repo_path);
    let latest_tags = get_latest_tags_by_scope(repo_path);

    writeln!(writer, "发布状态")?;
    writeln!(writer, "{}", "─".repeat(40))?;

    if latest_tags.is_empty() {
        writeln!(writer, "  最新标签:     (无)")?;
        return Ok(());
    }

    for (scope, tag) in &latest_tags {
        let scope_dir = if scope == "(root)" {
            repo_path.to_path_buf()
        } else {
            match scopes_map.get(scope) {
                Some(rel) => repo_path.join(rel),
                None => {
                    let d = repo_path.join(scope);
                    if d.is_dir() { d } else { repo_path.to_path_buf() }
                }
            }
        };

        writeln!(writer, "  [{}]", scope)?;
        let rel_path = scopes_map.get(scope).cloned().unwrap_or_else(|| {
            if scope == "(root)" { ".".into() } else { scope.clone() }
        });
        writeln!(writer, "    路径:         {}", rel_path)?;
        writeln!(writer, "    最新标签:     {}", tag)?;

        let unreleased = count_unreleased_in_dir(repo_path, tag, &scope_dir);
        writeln!(writer, "    未发布提交:   {}", unreleased)?;
    }

    Ok(())
}

fn load_scopes_map(repo_path: &Path) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = contract::load_scopes(repo_path)
        .into_iter()
        .map(|s| (s.name, s.dir))
        .collect();
    if !map.contains_key("(root)") {
        map.insert("(root)".to_string(), "".to_string());
    }
    map
}

fn get_latest_tags_by_scope(repo_path: &Path) -> Vec<(String, String)> {
    let out = match std::process::Command::new("git")
        .args(["tag", "--list"])
        .current_dir(repo_path)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    if !out.status.success() {
        return vec![];
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut tags: Vec<&str> = stdout.lines().collect();
    tags.sort_by(|a, b| {
        let a_ver = crate::release::util::parse_tag_semver(a);
        let b_ver = crate::release::util::parse_tag_semver(b);
        b_ver.cmp(&a_ver)
    });
    collect_latest_tags(&tags)
}

pub fn collect_latest_tags(tags: &[&str]) -> Vec<(String, String)> {
    let mut scopes: Vec<(String, String)> = Vec::new();
    for t in tags {
        let scope = if t.contains('/') {
            t.split('/').next().unwrap_or("").to_string()
        } else {
            "(root)".to_string()
        };
        if !scopes.iter().any(|(s, _)| s == &scope) {
            scopes.push((scope, t.to_string()));
        }
    }
    scopes
}

fn count_unreleased_in_dir(repo_path: &Path, tag: &str, scope_dir: &Path) -> usize {
    if is_git_repo(scope_dir) {
        return count_unreleased_in_submodule(scope_dir, tag);
    }
    let rel = scope_dir.strip_prefix(repo_path).unwrap_or(scope_dir);
    let rel_str = rel.to_string_lossy().trim_start_matches('/').to_string();
    let range = format!("{}..HEAD", tag);
    let mut args = vec!["rev-list", "--count", &range];
    if !rel_str.is_empty() && rel_str != "." {
        args.push("--");
        args.push(rel_str.as_str());
    }
    std::process::Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                s.parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn is_git_repo(path: &Path) -> bool {
    let git_dir = path.join(".git");
    git_dir.is_dir() || git_dir.is_file()
}

fn count_unreleased_in_submodule(submodule_path: &Path, tag: &str) -> usize {
    std::process::Command::new("git")
        .args(["rev-list", "--count", &format!("{}..HEAD", tag)])
        .current_dir(submodule_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                s.parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_tags_empty() {
        assert!(collect_latest_tags(&[]).is_empty());
    }

    #[test]
    fn test_collect_tags_root_only() {
        let tags = collect_latest_tags(&["v2.0.0", "v1.0.0"]);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].0, "(root)");
        assert_eq!(tags[0].1, "v2.0.0");
    }

    #[test]
    fn test_collect_tags_scoped() {
        let tags = collect_latest_tags(&["cli/v0.1.0", "web/v0.2.0"]);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].0, "cli");
        assert_eq!(tags[1].0, "web");
    }

    #[test]
    fn test_get_latest_tags_semver_v10_greater_than_v9() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(d.path().join("f"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["tag", "v9.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["tag", "v10.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        let tags = get_latest_tags_by_scope(d.path());
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].1, "v10.0.0", "v10.0.0 应在 v9.0.0 之前");
    }

    #[test]
    fn test_collect_tags_prerelease_is_kept() {
        let tags = collect_latest_tags(&["cli/v0.2.0-rc.1", "cli/v0.1.0"]);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].1, "cli/v0.2.0-rc.1");
    }

    #[test]
    fn test_collect_tags_prerelease_as_fallback() {
        let tags = collect_latest_tags(&["cli/v0.1.0-rc.2", "cli/v0.1.0-rc.1"]);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].1, "cli/v0.1.0-rc.2");
    }

    // ── 测试辅助 ────────────────────────────────────────────────

    fn git_init_test(path: &Path) {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
            .current_dir(path)
            .output()
            .unwrap();
        std::fs::write(path.join("f"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_tag_test(path: &Path, tag: &str) {
        std::process::Command::new("git")
            .args(["-C", path.to_str().unwrap(), "tag", tag])
            .output()
            .unwrap();
    }

    #[test]
    fn test_collect_tags_mixed_root_and_scoped() {
        let tags = collect_latest_tags(&["v1.0.0", "cli/v0.2.0", "cli/v0.1.0"]);
        assert_eq!(tags.len(), 2);
        let root = tags.iter().find(|(s, _)| s == "(root)").unwrap();
        assert_eq!(root.1, "v1.0.0");
        let cli = tags.iter().find(|(s, _)| s == "cli").unwrap();
        assert_eq!(cli.1, "cli/v0.2.0");
    }

    // ── is_git_repo ───────────────────────────────────────────

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

    // ── collect_latest_tags 性能 ────────────────────────────

    #[test]
    fn test_collect_latest_tags_large_input() {
        use std::time::Instant;
        let mut tags: Vec<&str> = Vec::with_capacity(10000);
        for i in 0..5000 {
            tags.push(Box::leak(
                format!("cli/v0.{}.{}", i / 100, i % 100).into_boxed_str(),
            ));
            tags.push(Box::leak(
                format!("sdk/v0.{}.{}", i / 100, i % 100).into_boxed_str(),
            ));
        }
        tags.sort_by(|a, b| b.cmp(a));
        let start = Instant::now();
        let result = collect_latest_tags(&tags);
        let elapsed = start.elapsed();
        assert_eq!(result.len(), 2, "两个 scope 各有最新 tag");
        assert!(
            elapsed.as_micros() < 10_000,
            "10000 tag 排序应 < 10ms，实际: {}μs",
            elapsed.as_micros()
        );
    }

    #[test]
    fn test_status_to_output() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(d.path().join("f"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();

        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok(), "status_to 应成功: {:?}", result);
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("发布状态"), "应包含标题");
        assert!(out.contains("v1.0.0"), "应包含 tag 信息");
    }
}
