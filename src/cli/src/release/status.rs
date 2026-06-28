use std::collections::HashMap;
use std::path::Path;

pub fn status(repo_path: &Path) {
    let scopes_map = read_contract_scopes(repo_path);
    let latest_tags = get_latest_tags_by_scope(repo_path);
    let dirty = is_dirty(repo_path);

    let other_scope_dirs: Vec<std::path::PathBuf> = scopes_map
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .map(|(_, v)| repo_path.join(v))
        .collect();

    println!("发布状态");
    println!("{}", "─".repeat(40));

    if latest_tags.is_empty() {
        println!("  最新标签:     (无)");
        return;
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

        println!("  [{}]", scope);
        let rel_path = scopes_map.get(scope).cloned().unwrap_or_else(|| {
            if scope == "(root)" {
                ".".to_string()
            } else {
                scope.clone()
            }
        });
        println!("    路径:         {}", rel_path);
        println!("    最新标签:     {}", tag);

        let unreleased = count_unreleased_in_dir(repo_path, tag, &scope_dir);
        println!("    未发布提交:   {}", unreleased);

        if check_changelog(&scope_dir, ver) {
            println!("    CHANGELOG:    ✅");
        } else {
            println!("    CHANGELOG:    ❌ 缺少 {} 条目", ver);
        }

        check_github_release(repo_path, tag, &scope_dir, ver);
        check_all_configs(&scope_dir, &other_scope_dirs, ver);
    }

    if dirty {
        println!("  工作区:       ❌ 有未提交变更");
    } else {
        println!("  工作区:       ✅ 干净");
    }
}

/// 检查 GitHub Release 是否存在，以及 body 是否与 CHANGELOG 同步。
fn check_github_release(repo_path: &Path, tag: &str, scope_dir: &Path, _version: &str) {
    // 解析 GitHub 仓库
    let repo = get_github_repo(repo_path);
    let repo = match repo {
        Some(r) => r,
        None => return,
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
            println!("    GitHub Release: ❌ 不存在");
            return;
        }
    };

    // 从 CHANGELOG 提取当前版本的 notes
    let changelog_path = scope_dir.join("CHANGELOG.md");
    let notes = super::util::extract_notes(tag, &changelog_path);
    let notes = notes.unwrap_or_default();

    if body == notes {
        println!("    GitHub Release: ✅ body 与 CHANGELOG 一致");
    } else if body.trim().is_empty() {
        println!("    GitHub Release: ⚠️ body 为空");
    } else if notes.is_empty() {
        println!("    GitHub Release: ✅ 已创建 (CHANGELOG 无此版本条目)");
    } else {
        println!("    GitHub Release: ⚠️ body 与 CHANGELOG 不同步");
    }
}

fn read_contract_scopes(repo_path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let content = std::fs::read_to_string(repo_path.join(".quanttide/devops/contract.yaml"))
        .unwrap_or_default();
    let mut in_scopes = false;
    for line in content.lines() {
        let t = line.trim();
        if t == "scopes:" {
            in_scopes = true;
            continue;
        }
        if in_scopes {
            if t.starts_with('#') || t.is_empty() {
                continue;
            }
            if !t.starts_with('-') && t.contains(':') {
                if let Some(idx) = t.find(':') {
                    let key = t[..idx].trim().to_string();
                    let val = t[idx + 1..].trim().to_string();
                    if !key.is_empty() {
                        map.insert(key, val);
                    }
                }
            } else if !t.starts_with(' ') && !t.starts_with('-') {
                break;
            }
        }
    }
    if !map.contains_key("(root)") {
        map.insert("(root)".to_string(), ".".to_string());
    }
    map
}

fn get_latest_tags_by_scope(repo_path: &Path) -> Vec<(String, String)> {
    let out = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "tag",
            "--sort=-version:refname",
        ])
        .output()
        .ok();
    let out = match out {
        Some(o) if o.status.success() => o,
        _ => return vec![],
    };
    let all: Vec<&str> = std::str::from_utf8(&out.stdout)
        .unwrap_or("")
        .lines()
        .collect();
    let mut scopes: Vec<(String, String)> = Vec::new();
    for t in all {
        let scope = if t.contains('/') {
            t.split('/').next().unwrap_or("").to_string()
        } else {
            "(root)".to_string()
        };
        let tag_only = t.split('/').last().unwrap_or(t);
        let pre = tag_only.contains('-');
        if let Some(pos) = scopes.iter().position(|(s, _)| s == &scope) {
            let et = scopes[pos].1.split('/').last().unwrap_or(&scopes[pos].1);
            if !pre && et.contains('-') {
                scopes[pos] = (scope, t.to_string());
            }
        } else {
            scopes.push((scope, t.to_string()));
        }
    }
    scopes
}

fn count_unreleased_in_dir(repo_path: &Path, tag: &str, scope_dir: &Path) -> usize {
    let range = format!("{}..HEAD", tag);
    if scope_dir == repo_path {
        let out = std::process::Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "rev-list",
                "--count",
                &range,
            ])
            .output()
            .ok();
        return match out {
            Some(o) if o.status.success() => std::str::from_utf8(&o.stdout)
                .unwrap_or("0")
                .trim()
                .parse()
                .unwrap_or(0),
            _ => 0,
        };
    }
    let rel = scope_dir.strip_prefix(repo_path).unwrap_or(scope_dir);
    let rel_str = rel.to_string_lossy().trim_start_matches('/').to_string();
    let out = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "rev-list",
            "--count",
            &range,
            "--",
            &rel_str,
        ])
        .output()
        .ok();
    match out {
        Some(o) if o.status.success() => std::str::from_utf8(&o.stdout)
            .unwrap_or("0")
            .trim()
            .parse()
            .unwrap_or(0),
        _ => 0,
    }
}

fn get_github_repo(repo_path: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "remote",
            "get-url",
            "origin",
        ])
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

fn check_all_configs(repo_path: &Path, other_scope_dirs: &[std::path::PathBuf], expected: &str) {
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
            Some(v) if v == expected => println!("    {:<15} {} ✅", format!("{}:", name), v),
            Some(v) => println!(
                "    {:<15} {} ❌ (期望 {})",
                format!("{}:", name),
                v,
                expected
            ),
            None => println!("    {:<15} (未找到版本字段)", format!("{}:", name)),
        }
    }
    let vf = repo_path.join("VERSION");
    if let Ok(c) = std::fs::read_to_string(&vf) {
        let v = c.trim().to_string();
        if !v.is_empty() {
            if v == expected {
                println!("    VERSION          {} ✅", v);
            } else {
                println!("    VERSION          {} ❌ (期望 {})", v, expected);
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
                                println!("    {:<15} {} ✅", format!("{}:", name), v);
                            } else {
                                println!(
                                    "    {:<15} {} ❌ (期望 {})",
                                    format!("{}:", name),
                                    v,
                                    expected
                                );
                            }
                        }
                    }
                }
            }
        }
    }
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
        if let Some(r) = t.strip_prefix("\"version\":") {
            let v = r
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim_matches(',');
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
    let out = std::process::Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .ok();
    match out {
        Some(o) => !o.stdout.is_empty(),
        None => false,
    }
}
