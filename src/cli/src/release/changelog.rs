use std::path::Path;

use quanttide_agent::{llm::CompleteOptions, Message, Settings, LLM};

/// 收集上个 tag 到当前 HEAD 之间的 git 提交记录。
pub fn collect_git_log(repo_path: &Path) -> Result<String, String> {
    let last_tag = get_latest_tag(repo_path);
    let range = match &last_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => "--all".to_string(),
    };
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "--no-decorate", &range])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("git log 执行失败: {}", e))?;
    if !output.status.success() {
        return Err("git log 返回非零退出码".into());
    }
    let log = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if log.is_empty() {
        return Err("没有新的提交记录".into());
    }
    Ok(log)
}

/// 获取仓库中最新版本 tag（按版本排序取最后一个）。
fn get_latest_tag(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["tag", "--sort=-version:refname"])
        .current_dir(repo_path)
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

/// 调用 LLM 生成 CHANGELOG 条目。
///
/// 通过 quanttide-agent 调用 LLM。如果 LLM 未配置（LLM_API_KEY 为空），
/// 返回提示文本让用户手动发送给 AI（不阻塞发布流程）。
fn llm_changelog(git_log: &str, version: &str) -> Result<String, String> {
    let hint = format!(
        "根据以下 git 提交记录生成 CHANGELOG 条目，按 Added / Changed / Fixed / Removed 分类：\n\n\
         提交记录：\n{git_log}\n\n版本号：{version}"
    );

    let settings = Settings::from_env();
    if settings.llm_api_key.is_empty() {
        return Err(format!(
            "LLM 未配置（LLM_API_KEY 未设置）。请将以下文本发送给 AI 生成 CHANGELOG：\n\n{hint}"
        ));
    }

    let llm = LLM::new(
        &settings.llm_model,
        &settings.llm_base_url,
        &settings.llm_api_key,
    );
    let messages = vec![
        Message::new(
            "system",
            "你是一个帮助生成 CHANGELOG 的助手。\
             严格按 Keep a Changelog 格式输出，分类为 Added / Changed / Fixed / Removed。\
             只输出分类后的条目内容，不要输出版本头部（## 行）和日期。",
        ),
        Message::new("user", &hint),
    ];
    let response = llm
        .complete(&messages, CompleteOptions::default())
        .map_err(|e| format!("LLM 调用失败: {}\n\n请手动生成 CHANGELOG：\n\n{hint}", e))?;

    Ok(response.content.trim().to_string())
}

/// 将生成的 CHANGELOG 条目写入文件。
pub fn write_changelog(path: &Path, version: &str, content: &str) -> Result<(), String> {
    let ver = super::util::normalize_version(version);
    let entry = format!("\n## [{}] - {}\n\n{}\n", ver, today(), content);
    let mut existing = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| format!("读取 CHANGELOG.md 失败: {}", e))?
    } else {
        "# CHANGELOG\n".to_string()
    };
    if let Some(pos) = existing.find("\n## ") {
        existing.insert_str(pos, &entry);
    } else {
        existing.push_str(&entry);
    }
    std::fs::write(path, &existing).map_err(|e| format!("写入 CHANGELOG.md 失败: {}", e))?;
    Ok(())
}

fn today() -> String {
    // 不使用 chrono，用 git 的方式获取当前日期
    let output = std::process::Command::new("date")
        .args(["+%Y-%m-%d"])
        .output()
        .ok();
    output
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// 如果 CHANGELOG.md 不包含当前版本，则自动生成并写入。
pub fn ensure_changelog(repo_path: &Path, version: &str) -> Result<(), String> {
    let changelog_path = repo_path.join("CHANGELOG.md");
    if changelog_path.exists() {
        let content = std::fs::read_to_string(&changelog_path)
            .map_err(|e| format!("读取 CHANGELOG.md 失败: {}", e))?;
        let ver = super::util::normalize_version(version);
        if content.contains(&format!("[{}]", ver)) || content.contains(&format!("[v{}]", ver)) {
            return Ok(());
        }
    }
    let git_log = collect_git_log(repo_path)?;
    let changelog_content = llm_changelog(&git_log, version)?;
    write_changelog(&changelog_path, version, &changelog_content)?;
    println!("✓ CHANGELOG.md 已更新（版本 {})", version);

    // 提交 CHANGELOG 修改，确保后续标签包含它
    let ver = super::util::normalize_version(version);
    let add = std::process::Command::new("git")
        .args(["add", "CHANGELOG.md"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("git add 失败: {}", e))?;
    if !add.status.success() {
        return Err("git add CHANGELOG.md 失败".into());
    }
    let commit = std::process::Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("chore: add CHANGELOG entry for {}", ver),
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("git commit 失败: {}", e))?;
    if !commit.status.success() {
        return Err("git commit CHANGELOG.md 失败".into());
    }
    println!("✓ CHANGELOG 修改已提交");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_init(path: &Path) {
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
    }

    fn git_commit(path: &Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", msg])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_collect_git_log_with_commits() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "feat: add foo");
        git_commit(d.path(), "fix: bar");
        let log = collect_git_log(d.path()).unwrap();
        assert!(log.contains("feat: add foo"));
    }

    #[test]
    fn test_collect_git_log_single_commit() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        let log = collect_git_log(d.path()).unwrap();
        assert!(log.contains("init"));
    }

    #[test]
    fn test_write_changelog_creates_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("CHANGELOG.md");
        write_changelog(&path, "v0.1.0", "### Added\n- new feature").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("## [0.1.0]"));
    }

    #[test]
    fn test_write_changelog_append() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("CHANGELOG.md");
        std::fs::write(&path, "# CHANGELOG\n\n## [0.1.0]\n\nold\n").unwrap();
        write_changelog(&path, "v0.2.0", "### Added\n- new").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("## [0.2.0]"));
        assert!(content.contains("## [0.1.0]"));
    }

    #[test]
    fn test_ensure_changelog_skips_if_exists() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("CHANGELOG.md");
        std::fs::write(&path, "# CHANGELOG\n\n## [0.1.0]\n\ncontent\n").unwrap();
        assert!(ensure_changelog(d.path(), "v0.1.0").is_ok());
    }

    #[test]
    fn test_ensure_changelog_no_git_log() {
        let d = tempfile::tempdir().unwrap();
        let result = ensure_changelog(d.path(), "v0.1.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_latest_tag_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        assert!(get_latest_tag(d.path()).is_none());
    }

    #[test]
    fn test_latest_tag_with_tags() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        std::process::Command::new("git")
            .args(["tag", "v0.1.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["tag", "v0.2.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert_eq!(get_latest_tag(d.path()).as_deref(), Some("v0.2.0"));
    }
}
