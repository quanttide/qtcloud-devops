use std::path::Path;

/// 显示发布状态信息。
pub fn status(repo_path: &Path) {
    let version = read_version(repo_path, "Cargo.toml");
    let pyproject_ver = read_version(repo_path, "pyproject.toml");
    let latest_tag = get_latest_tag(repo_path);
    let unreleased = count_unreleased(repo_path, &latest_tag);
    let changelog_ok = check_changelog(repo_path, &version);
    let dirty = is_dirty(repo_path);

    println!("发布状态");
    println!("{}", "─".repeat(40));
    println!("  当前版本:     {}", version);
    if pyproject_ver == version {
        println!("  pyproject:    {} ✅", pyproject_ver);
    } else {
        println!("  pyproject:    {} ❌ (期望 {})", pyproject_ver, version);
    }
    match &latest_tag {
        Some(t) => println!("  最新标签:     {}", t),
        None => println!("  最新标签:     (无)"),
    }
    println!("  未发布提交:   {}", unreleased);
    if changelog_ok {
        println!("  CHANGELOG:    ✅ 条目存在");
    } else {
        println!("  CHANGELOG:    ❌ 缺少 {} 条目", version);
    }
    if dirty {
        println!("  工作区:       ❌ 有未提交变更");
    } else {
        println!("  工作区:       ✅ 干净");
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
    "unknown".to_string()
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
    std::str::from_utf8(&output.stdout)
        .ok()?
        .lines()
        .next()
        .map(|s| s.to_string())
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
    let path = repo_path.join("CHANGELOG.md");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    // 支持 [0.1.0] 和 [v0.1.0] 两种格式
    let ver = version.strip_prefix('v').unwrap_or(version);
    content.contains(&format!("[{}]", ver))
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
