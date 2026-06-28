use std::path::Path;

/// 显示发布状态信息。事实源是 git tag，配置文件版本应与 tag 一致。
pub fn status(repo_path: &Path) {
    let latest_tag = get_latest_tag(repo_path);
    let tag_ver = normalize_tag(&latest_tag);
    let dirty = is_dirty(repo_path);

    println!("发布状态");
    println!("{}", "─".repeat(40));

    match &latest_tag {
        Some(t) => {
            let unreleased = count_unreleased(repo_path, &latest_tag);
            let changelog_ok = check_changelog(repo_path, &tag_ver);

            println!("  最新标签:     {}", t);
            println!("  未发布提交:   {}", unreleased);
            check_config_version(repo_path, "Cargo.toml", &tag_ver);
            if repo_path.join("pyproject.toml").exists() {
                check_config_version(repo_path, "pyproject.toml", &tag_ver);
            }
            if changelog_ok {
                println!("  CHANGELOG:    ✅ 条目存在");
            } else {
                println!("  CHANGELOG:    ❌ 缺少 {} 条目", tag_ver);
            }
        }
        None => {
            println!("  最新标签:     (无)");
            println!("  Cargo.toml:   {}", read_version(repo_path, "Cargo.toml"));
        }
    }

    if dirty {
        println!("  工作区:       ❌ 有未提交变更");
    } else {
        println!("  工作区:       ✅ 干净");
    }
}

/// 从 tag 中提取版本号（去掉 scope 前缀和 v 前缀）。
fn normalize_tag(tag: &Option<String>) -> String {
    match tag {
        Some(t) => {
            let s = t.split('/').last().unwrap_or(t);
            s.strip_prefix('v').unwrap_or(s).to_string()
        }
        None => String::new(),
    }
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

fn check_config_version(repo_path: &Path, filename: &str, expected: &str) {
    let actual = read_version(repo_path, filename);
    if actual == *expected {
        println!("  {:<14} {} ✅", format!("{}:", filename), actual);
    } else if actual.is_empty() {
        println!("  {:<14} ❌ 文件不存在或无法解析", format!("{}:", filename));
    } else {
        println!(
            "  {:<14} {} ❌ (期望 {})",
            format!("{}:", filename),
            actual,
            expected
        );
    }
}

fn get_latest_tag(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "tag",
            "--sort=-version:refname",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tags: Vec<&str> = std::str::from_utf8(&output.stdout).ok()?.lines().collect();
    // 先找最新的正式版 tag（不含 -rc / -alpha / -beta）
    for t in &tags {
        let ver = t.split('/').last().unwrap_or(t);
        if !ver.contains('-') {
            return Some(t.to_string());
        }
    }
    tags.first().map(|s| s.to_string())
}

fn count_unreleased(repo_path: &Path, latest_tag: &Option<String>) -> usize {
    let range = match latest_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => "--all".to_string(),
    };
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
