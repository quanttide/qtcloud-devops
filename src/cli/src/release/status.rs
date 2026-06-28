use std::path::Path;

/// 显示发布状态信息。事实源是 git tag，配置文件版本应与 tag 一致。
pub fn status(repo_path: &Path) {
    let latest_tags = get_latest_tags_by_scope(repo_path);
    let cargo_ver = read_version(repo_path, "Cargo.toml");
    let dirty = is_dirty(repo_path);

    println!("发布状态");
    println!("{}", "─".repeat(40));

    if latest_tags.is_empty() {
        println!("  最新标签:     (无)");
        println!("  Cargo.toml:   {}", cargo_ver);
    } else {
        for (scope, tag) in &latest_tags {
            let tag_only = tag.split('/').last().unwrap_or(tag);
            let ver = tag_only.strip_prefix('v').unwrap_or(tag_only);
            if ver == &cargo_ver {
                // 匹配当前仓库，显示详细信息
                let unreleased = count_unreleased(repo_path, tag);
                let changelog_ok = check_changelog(repo_path, ver);
                println!("  [{}]", scope);
                println!("    最新标签:     {}", tag);
                println!("    未发布提交:   {}", unreleased);
                check_config_version(repo_path, "Cargo.toml", ver, true);
                if repo_path.join("pyproject.toml").exists() {
                    check_config_version(repo_path, "pyproject.toml", ver, true);
                }
                if changelog_ok {
                    println!("    CHANGELOG:    ✅");
                } else {
                    println!("    CHANGELOG:    ❌ 缺少 {} 条目", ver);
                }
            } else {
                // 不匹配，简略显示
                println!("  [{}]     {}", scope, tag);
            }
        }
    }

    if dirty {
        println!("  工作区:       ❌ 有未提交变更");
    } else {
        println!("  工作区:       ✅ 干净");
    }
}

/// 按 scope 分组，取每个 scope 最新的 tag。
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
            // 已有该 scope 的 tag，如果当前是正式版且已有的是预发布版，替换
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

fn read_version(repo_path: &Path, filename: &str) -> String {
    let path = repo_path.join(filename);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(ver) = trimmed.strip_prefix("version = \"") {
            if let Some(end) = ver.find('"') {
                return ver[..end].to_string();
            }
        }
    }
    String::new()
}

fn check_config_version(repo_path: &Path, filename: &str, expected: &str, matched: bool) {
    let actual = read_version(repo_path, filename);
    if actual == *expected {
        println!("    {:<12} {} ✅", format!("{}:", filename), actual);
    } else if actual.is_empty() {
        if matched {
            println!("    {:<12} ❌ 无法解析", format!("{}:", filename));
        }
    } else {
        println!(
            "    {:<12} {} ❌ (期望 {})",
            format!("{}:", filename),
            actual,
            expected
        );
    }
}

fn count_unreleased(repo_path: &Path, tag: &str) -> usize {
    let range = format!("{}..HEAD", tag);
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
    match output {
        Some(o) if o.status.success() => std::str::from_utf8(&o.stdout)
            .unwrap_or("0")
            .trim()
            .parse()
            .unwrap_or(0),
        _ => 0,
    }
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
