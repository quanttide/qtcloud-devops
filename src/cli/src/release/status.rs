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
    let dirty = is_dirty(repo_path);

    let other_scope_dirs: Vec<std::path::PathBuf> = scopes_map
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .map(|(_, v)| repo_path.join(v))
        .collect();

    writeln!(writer, "发布状态")?;
    writeln!(writer, "{}", "─".repeat(40))?;

    if latest_tags.is_empty() {
        writeln!(writer, "  最新标签:     (无)")?;
        return Ok(());
    }

    for (scope, tag) in &latest_tags {
        let tag_only = tag.split('/').last().unwrap_or(tag);
        let ver = tag_only.strip_prefix('v').unwrap_or(tag_only);

        let scope_dir = if scope == "(root)" {
            repo_path.to_path_buf()
        } else {
            match scopes_map.get(scope) {
                Some(rel) => repo_path.join(rel),
                None => {
                    let d = repo_path.join(scope);
                    if d.is_dir() {
                        d
                    } else {
                        repo_path.to_path_buf()
                    }
                }
            }
        };

        writeln!(writer, "  [{}]", scope)?;
        let rel_path = scopes_map.get(scope).cloned().unwrap_or_else(|| {
            if scope == "(root)" {
                ".".to_string()
            } else {
                scope.clone()
            }
        });
        writeln!(writer, "    路径:         {}", rel_path)?;
        writeln!(writer, "    最新标签:     {}", tag)?;

        let unreleased = count_unreleased_in_dir(repo_path, tag, &scope_dir);
        writeln!(writer, "    未发布提交:   {}", unreleased)?;

        if check_changelog(&scope_dir, ver) {
            writeln!(writer, "    CHANGELOG:    ✅")?;
        } else {
            writeln!(writer, "    CHANGELOG:    ❌ 缺少 {} 条目", ver)?;
        }

        check_github_release(writer, repo_path, tag, &scope_dir, ver)?;
        check_all_configs(writer, &scope_dir, &other_scope_dirs, ver)?;
    }

    if dirty {
        writeln!(writer, "  工作区:       ❌ 有未提交变更")?;
    } else {
        writeln!(writer, "  工作区:       ✅ 干净")?;
    }

    Ok(())
}

/// 检查 GitHub Release 是否存在，以及 body 是否与 CHANGELOG 同步。
fn check_github_release(
    writer: &mut impl std::io::Write,
    repo_path: &Path,
    tag: &str,
    scope_dir: &Path,
    _version: &str,
) -> std::io::Result<()> {
    // 解析 GitHub 仓库
    let repo = get_github_repo(repo_path);
    let repo = match repo {
        Some(r) => r,
        None => return Ok(()),
    };

    // 查询 Release
    let out = std::process::Command::new("gh")
        .args([
            "release", "view", tag, "--repo", &repo, "--json", "body", "--jq", ".body",
        ])
        .output()
        .ok();

    let body = match out {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => {
            writeln!(writer, "    GitHub Release: ❌ 不存在")?;
            return Ok(());
        }
    };

    // 从 CHANGELOG 提取当前版本的 notes
    let changelog_path = scope_dir.join("CHANGELOG.md");
    let notes = super::util::extract_notes(tag, &changelog_path);
    let notes = notes.unwrap_or_default();

    if body == notes {
        writeln!(writer, "    GitHub Release: ✅ body 与 CHANGELOG 一致")?;
    } else if body.trim().is_empty() {
        writeln!(writer, "    GitHub Release: ⚠️ body 为空")?;
    } else if notes.is_empty() {
        writeln!(
            writer,
            "    GitHub Release: ✅ 已创建 (CHANGELOG 无此版本条目)"
        )?;
    } else {
        writeln!(writer, "    GitHub Release: ⚠️ body 与 CHANGELOG 不同步")?;
    }

    Ok(())
}

/// 从契约加载 scope 列表，转为 (name → dir) 映射。
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
    let out = std::process::Command::new("git")
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
        .unwrap_or(0);
    out
}

/// 检测路径是否为 git 仓库
fn is_git_repo(path: &Path) -> bool {
    let git_dir = path.join(".git");
    git_dir.is_dir() || git_dir.is_file()
}

/// 在子模组内统计未发布提交数（子模组自己的 tag 和 HEAD）
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

fn get_github_repo(repo_path: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = std::str::from_utf8(&out.stdout).ok()?.trim().to_string();
    let re = regex::Regex::new(r"github\.com[/:]([^/]+/[^/]+?)(?:\.git)?$").ok()?;
    let caps = re.captures(&url)?;
    Some(caps.get(1)?.as_str().to_string())
}

fn check_all_configs(
    writer: &mut impl std::io::Write,
    repo_path: &Path,
    other_scope_dirs: &[std::path::PathBuf],
    expected: &str,
) -> std::io::Result<()> {
    let checks: [(&str, fn(&str) -> Option<String>); 5] = [
        ("Cargo.toml", |c| extract_kv(c, "version")),
        ("pyproject.toml", |c| extract_kv(c, "version")),
        ("package.json", extract_json_version),
        ("pubspec.yaml", |c| extract_kv_yaml(c, "version")),
        ("setup.cfg", |c| extract_kv(c, "version")),
    ];
    for (name, extract) in &checks {
        let content = match std::fs::read_to_string(&repo_path.join(name)) {
            Ok(c) => c,
            Err(_) => continue,
        };
        match extract(&content) {
            Some(v) if v == expected => {
                writeln!(writer, "    {:<15} {} ✅", format!("{}:", name), v)?
            }
            Some(v) => writeln!(
                writer,
                "    {:<15} {} ❌ (期望 {})",
                format!("{}:", name),
                v,
                expected
            )?,
            None => writeln!(writer, "    {:<15} (未找到版本字段)", format!("{}:", name))?,
        }
    }
    let vf = repo_path.join("VERSION");
    if let Ok(c) = std::fs::read_to_string(&vf) {
        let v = c.trim().to_string();
        if !v.is_empty() {
            if v == expected {
                writeln!(writer, "    VERSION          {} ✅", v)?;
            } else {
                writeln!(writer, "    VERSION          {} ❌ (期望 {})", v, expected)?;
            }
        }
    }
    for p in find_go_files(repo_path, other_scope_dirs) {
        let content = match std::fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for prefix in &[
            "var Version = \"",
            "var VERSION = \"",
            "const Version = \"",
            "const VERSION = \"",
        ] {
            for line in content.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix(prefix) {
                    if let Some(end) = rest.find('"') {
                        let v = rest[..end].to_string();
                        if !v.is_empty() {
                            let rel = p.strip_prefix(repo_path).unwrap_or(&p);
                            let name = rel.to_string_lossy();
                            if v == expected {
                                writeln!(writer, "    {:<15} {} ✅", format!("{}:", name), v)?;
                            } else {
                                writeln!(
                                    writer,
                                    "    {:<15} {} ❌ (期望 {})",
                                    format!("{}:", name),
                                    v,
                                    expected
                                )?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn find_go_files(dir: &Path, excludes: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return files,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if excludes.iter().any(|e| p == *e) {
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.')
                && name != "node_modules"
                && name != "target"
                && name != "vendor"
            {
                files.extend(find_go_files(&p, excludes));
            }
        } else if p.extension().and_then(|e| e.to_str()) == Some("go") {
            files.push(p);
        }
    }
    files
}

fn extract_kv(content: &str, key: &str) -> Option<String> {
    let p1 = format!("{} = \"", key);
    let p2 = format!("{} = '", key);
    for line in content.lines() {
        let t = line.trim();
        if let Some(r) = t.strip_prefix(&p1) {
            if let Some(e) = r.find('"') {
                let v = r[..e].to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
        if let Some(r) = t.strip_prefix(&p2) {
            if let Some(e) = r.find('\'') {
                let v = r[..e].to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn extract_json_version(content: &str) -> Option<String> {
    for line in content.lines() {
        let t = line.trim();
        // 用 find 而非 strip_prefix，支持单行 JSON（"version": 不在行首）
        if let Some(pos) = t.find("\"version\":") {
            let after_colon = t[pos + "\"version\":".len()..].trim();
            // 定位第一个引号 -> value 起点
            let value_start = after_colon.find('"')?;
            let after_open = &after_colon[value_start + 1..];
            // 定位闭合引号 -> value 终点
            let value_end = after_open.find('"')?;
            let v = &after_open[..value_end];
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn extract_kv_yaml(content: &str, key: &str) -> Option<String> {
    let p = format!("{}:", key);
    for line in content.lines() {
        let t = line.trim();
        if let Some(r) = t.strip_prefix(&p) {
            let v = r.trim();
            if !v.is_empty() && !v.starts_with('#') {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn check_changelog(repo_path: &Path, version: &str) -> bool {
    if version.is_empty() {
        return false;
    }
    std::fs::read_to_string(repo_path.join("CHANGELOG.md"))
        .unwrap_or_default()
        .contains(&format!("[{}]", version))
}

fn is_dirty(repo_path: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_kv ────────────────────────────────────────────

    #[test]
    fn test_extract_kv_double_quotes() {
        assert_eq!(
            extract_kv("version = \"1.0.0\"\n", "version"),
            Some("1.0.0".into())
        );
    }

    #[test]
    fn test_extract_kv_single_quotes() {
        assert_eq!(
            extract_kv("version = '2.0.0'\n", "version"),
            Some("2.0.0".into())
        );
    }

    #[test]
    fn test_extract_kv_missing_key() {
        assert_eq!(extract_kv("name = \"foo\"\n", "version"), None);
    }

    #[test]
    fn test_extract_kv_empty_value() {
        assert_eq!(extract_kv("version = \"\"\n", "version"), None);
    }

    #[test]
    fn test_extract_kv_indented() {
        assert_eq!(
            extract_kv("  version = \"0.5.0\"\n", "version"),
            Some("0.5.0".into())
        );
    }

    // ── extract_json_version ───────────────────────────────────

    #[test]
    fn test_extract_json_version_normal() {
        let content = "{\n  \"version\": \"1.0.0\",\n}\n";
        assert_eq!(extract_json_version(content), Some("1.0.0".into()));
    }

    #[test]
    fn test_extract_json_version_single_line() {
        let content = r#"{"name":"foo","version":"2.0.0"}"#;
        assert_eq!(extract_json_version(content), Some("2.0.0".into()));
    }

    #[test]
    fn test_extract_json_version_trailing_comma() {
        let content = r#"{"version":"1.0.0",}"#;
        assert_eq!(extract_json_version(content), Some("1.0.0".into()));
    }

    #[test]
    fn test_extract_json_version_missing() {
        let content = r#"{"name":"foo"}"#;
        assert_eq!(extract_json_version(content), None);
    }

    #[test]
    fn test_extract_json_version_empty() {
        let content = r#"{"version":""}"#;
        assert_eq!(extract_json_version(content), None);
    }

    // ── extract_kv_yaml ───────────────────────────────────────

    #[test]
    fn test_extract_kv_yaml_normal() {
        assert_eq!(
            extract_kv_yaml("version: 1.0.0\n", "version"),
            Some("1.0.0".into())
        );
    }

    #[test]
    fn test_extract_kv_yaml_indented() {
        assert_eq!(
            extract_kv_yaml("  version: 3.0.0\n", "version"),
            Some("3.0.0".into())
        );
    }

    #[test]
    fn test_extract_kv_yaml_ignores_comment() {
        assert_eq!(extract_kv_yaml("version: # 注释\n", "version"), None);
    }

    #[test]
    fn test_extract_kv_yaml_missing() {
        assert_eq!(extract_kv_yaml("name: foo\n", "version"), None);
    }

    #[test]
    fn test_extract_kv_yaml_empty_value() {
        assert_eq!(extract_kv_yaml("version:\n", "version"), None);
    }

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
        // 字符串排序 v9.0.0 > v10.0.0，语义化版本应有正确顺序
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
        // 输入已按版本降序，首个 tag 胜出（含 prerelease）
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

    /// 创建 mock bin 脚本并前置到 PATH，测试结束后还原。
    fn with_mock_path<F: FnOnce(&Path) -> R, R>(scripts: &[(&str, &str)], f: F) -> R {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        for (name, body) in scripts {
            let path = bin.join(name);
            std::fs::write(&path, body).unwrap();
            #[cfg(unix)]
            std::process::Command::new("chmod")
                .args(["+x", path.to_str().unwrap()])
                .output()
                .unwrap();
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", bin.display(), old_path));
        let result = f(dir.path());
        std::env::set_var("PATH", &old_path);
        result
    }

    const GH_NOT_FOUND: &str = "#!/bin/sh\nexit 1\n";
    const GH_WITH_BODY: &str = "#!/bin/sh\necho '{\"body\":\"content\"}'\n";

    #[test]
    fn test_status_gh_not_found() {
        // 有 GitHub remote 但 gh CLI 返回不存在 → 不 panic
        let dir = tempfile::tempdir().unwrap();
        git_init_test(dir.path());
        git_tag_test(dir.path(), "v1.0.0");
        set_remote(dir.path());
        with_mock_path(&[("gh", GH_NOT_FOUND)], |_| {
            status(dir.path());
        });
    }

    #[test]
    fn test_status_gh_with_body() {
        // gh 返回 body，CHANGELOG 匹配 → 一致
        let dir = tempfile::tempdir().unwrap();
        git_init_test(dir.path());
        std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
        git_commit_test(dir.path());
        git_tag_test(dir.path(), "v1.0.0");
        set_remote(dir.path());
        with_mock_path(&[("gh", GH_WITH_BODY)], |_| {
            status(dir.path());
        });
    }

    #[test]
    fn test_status_custom_tags() {
        // 自定义 mock git 返回的标签列表，覆盖多 scope 路径
        let dir = tempfile::tempdir().unwrap();
        // 用真实 git repo + 标签，验证 status 不 panic
        git_init_test(dir.path());
        git_tag_test(dir.path(), "cli/v0.1.0");
        git_tag_test(dir.path(), "web/v0.2.0");
        set_remote(dir.path());
        with_mock_path(&[("gh", GH_NOT_FOUND)], |_| {
            status(dir.path());
        });
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

    fn git_commit_test(path: &Path) {
        std::fs::write(path.join("f"), "x").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "x"])
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

    fn set_remote(path: &Path) {
        std::process::Command::new("git")
            .args([
                "-C",
                path.to_str().unwrap(),
                "remote",
                "add",
                "origin",
                "https://github.com/owner/repo.git",
            ])
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

    // ── check_changelog ────────────────────────────────────────

    #[test]
    fn test_check_changelog_found() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
        assert!(check_changelog(d.path(), "1.0.0"));
    }

    #[test]
    fn test_check_changelog_not_found() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## [0.9.0]\n\ncontent\n").unwrap();
        assert!(!check_changelog(d.path(), "1.0.0"));
    }

    #[test]
    fn test_check_changelog_no_file() {
        let d = tempfile::tempdir().unwrap();
        assert!(!check_changelog(d.path(), "1.0.0"));
    }

    #[test]
    fn test_check_changelog_empty_version() {
        let d = tempfile::tempdir().unwrap();
        assert!(!check_changelog(d.path(), ""));
    }

    // ── extract_kv 更多边缘 ─────────────────────────────────

    #[test]
    fn test_extract_kv_single_quotes_indented() {
        assert_eq!(
            extract_kv("  version = '1.2.3'\n", "version"),
            Some("1.2.3".into())
        );
    }

    #[test]
    fn test_extract_kv_double_quotes_trailing() {
        assert_eq!(
            extract_kv("version = \"1.0.0\"  # comment\n", "version"),
            Some("1.0.0".into())
        );
    }

    // ── extract_json_version 边缘 ────────────────────────────

    #[test]
    fn test_extract_json_version_nested() {
        let content = r#"{"scripts":{"test":"jest"},"version":"4.5.6"}"#;
        assert_eq!(extract_json_version(content), Some("4.5.6".into()));
    }

    // ── find_go_files ──────────────────────────────────────────

    #[test]
    fn test_find_go_files_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        let files = find_go_files(d.path(), &[]);
        assert!(files.is_empty());
    }

    #[test]
    fn test_find_go_files_finds_go() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("main.go"), "package main").unwrap();
        let files = find_go_files(d.path(), &[]);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.go"));
    }

    #[test]
    fn test_find_go_files_skips_excluded() {
        let d = tempfile::tempdir().unwrap();
        let sub = d.path().join("vendor");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("lib.go"), "package lib").unwrap();
        let files = find_go_files(d.path(), &[]);
        assert!(files.is_empty(), "vendor 目录应被跳过");
    }

    #[test]
    fn test_find_go_files_skips_custom_excludes() {
        let d = tempfile::tempdir().unwrap();
        let sub = d.path().join("generated");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("code.go"), "package code").unwrap();
        let files = find_go_files(d.path(), &[sub]);
        assert!(files.is_empty(), "自定义排除应有 0 个文件");
    }

    #[test]
    fn test_find_go_files_nested() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("pkg/util")).unwrap();
        std::fs::write(d.path().join("pkg/util/helper.go"), "package util").unwrap();
        let files = find_go_files(d.path(), &[]);
        assert_eq!(files.len(), 1);
    }

    // ── get_github_repo ───────────────────────────────────────

    #[test]
    fn test_get_github_repo_no_repo() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(get_github_repo(d.path()), None);
    }

    // ── status_to_output ───────────────────────────────────────

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
        // 初始化 git 仓库
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
        // 打一个 tag
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 写 CHANGELOG
        std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();

        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok(), "status_to 应成功: {:?}", result);
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("发布状态"), "应包含标题");
        assert!(out.contains("v1.0.0"), "应包含 tag 信息");
    }
}
