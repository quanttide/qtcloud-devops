use std::collections::HashMap;
use std::path::Path;

/// 显示发布状态信息。按 scope 分组，从 contract 读取子目录映射。
pub fn status(repo_path: &Path) {
    let scopes_map = read_contract_scopes(repo_path);
    let latest_tags = get_latest_tags_by_scope(repo_path);
    let dirty = is_dirty(repo_path);

    println!("发布状态");
    println!("{}", "─".repeat(40));

    if latest_tags.is_empty() {
        println!("  最新标签:     (无)");
        return;
    }

    for (scope, tag) in &latest_tags {
        let tag_only = tag.split('/').last().unwrap_or(tag);
        let ver = tag_only.strip_prefix('v').unwrap_or(tag_only);

        // scope 子目录：从 contract 读取，未配置时按 scopename 猜测
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

        // CHANGELOG 检查：只依赖 tag 版本，不依赖配置文件
        let changelog_ok = check_changelog(&scope_dir, ver);
        if changelog_ok {
            println!("    CHANGELOG:    ✅");
        } else {
            println!("    CHANGELOG:    ❌ 缺少 {} 条目", ver);
        }

        // 配置文件版本检查：辅助信息，文件不存在时不报错
        check_config(&scope_dir, ver);
    }

    if dirty {
        println!("  工作区:       ❌ 有未提交变更");
    } else {
        println!("  工作区:       ✅ 干净");
    }
}

fn read_contract_scopes(repo_path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let path = repo_path.join(".quanttide/devops/contract.yaml");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return map,
    };

    let mut in_scopes = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "scopes:" {
            in_scopes = true;
            continue;
        }
        if in_scopes {
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            if !trimmed.starts_with('-') && trimmed.contains(':') {
                if let Some(idx) = trimmed.find(':') {
                    let key = trimmed[..idx].trim().to_string();
                    let val = trimmed[idx + 1..].trim().to_string();
                    if !key.is_empty() {
                        map.insert(key, val);
                    }
                }
            } else if !trimmed.starts_with(' ') && !trimmed.starts_with('-') {
                break;
            }
        }
    }
    map
}

fn get_latest_tags_by_scope(repo_path: &Path) -> Vec<(String, String)> {
    let output = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "tag",
            "--sort=-version:refname",
        ])
        .output()
        .ok();
    let output = match output {
        Some(o) if o.status.success() => o,
        _ => return vec![],
    };
    let all_tags: Vec<&str> = std::str::from_utf8(&output.stdout)
        .unwrap_or("")
        .lines()
        .collect();

    let mut scopes: Vec<(String, String)> = Vec::new();
    for t in all_tags {
        let scope = if t.contains('/') {
            t.split('/').next().unwrap_or("").to_string()
        } else {
            "(root)".to_string()
        };
        let tag_only = t.split('/').last().unwrap_or(t);
        let is_prerelease = tag_only.contains('-');

        if let Some(pos) = scopes.iter().position(|(s, _)| s == &scope) {
            let existing = &scopes[pos].1;
            let existing_tag = existing.split('/').last().unwrap_or(existing);
            if !is_prerelease && existing_tag.contains('-') {
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
        let output = std::process::Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "rev-list",
                "--count",
                &range,
            ])
            .output()
            .ok();
        return match output {
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
    let output = std::process::Command::new("git")
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
    match output {
        Some(o) if o.status.success() => std::str::from_utf8(&o.stdout)
            .unwrap_or("0")
            .trim()
            .parse()
            .unwrap_or(0),
        _ => 0,
    }
}

fn check_config(repo_path: &Path, expected: &str) {
    for filename in &["Cargo.toml", "pyproject.toml", "package.json"] {
        let path = repo_path.join(filename);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let actual = extract_version(&content, filename);
        match actual {
            Some(v) if v == expected => {
                println!("    {:<12} {} ✅", format!("{}:", filename), v);
                return;
            }
            Some(v) => {
                println!(
                    "    {:<12} {} ❌ (期望 {})",
                    format!("{}:", filename),
                    v,
                    expected
                );
                return;
            }
            None => continue,
        }
    }
}

fn extract_version(content: &str, filename: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if filename == "Cargo.toml" || filename == "pyproject.toml" {
            if let Some(ver) = trimmed.strip_prefix("version = \"") {
                if let Some(end) = ver.find('"') {
                    return Some(ver[..end].to_string());
                }
            }
        }
        if filename == "package.json" {
            if let Some(rest) = trimmed.strip_prefix("\"version\":") {
                let v = rest.trim().trim_matches('"').trim_matches(',');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn check_changelog(repo_path: &Path, version: &str) -> bool {
    if version.is_empty() {
        return false;
    }
    let path = repo_path.join("CHANGELOG.md");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    content.contains(&format!("[{}]", version))
}

fn is_dirty(repo_path: &Path) -> bool {
    let output = std::process::Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .ok();
    match output {
        Some(o) => !o.stdout.is_empty(),
        None => false,
    }
}
