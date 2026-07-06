use std::path::Path;
use std::process::Command;

use quanttide_agent::{llm::CompleteOptions, Message, Settings, LLM};

#[derive(Debug, thiserror::Error)]
pub enum ChangelogError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("git 操作失败: {0}")]
    Git(String),
    #[error("LLM 调用失败: {0}")]
    Llm(String),
    #[error("没有新的提交记录")]
    NoNewCommits,
}

/// 收集上个 tag 到当前 HEAD 之间的 git 提交记录。
fn collect_git_log(repo_path: &Path) -> Result<String, ChangelogError> {
    let tag = get_latest_tag(repo_path);
    let range = match tag {
        Some(ref t) => format!("{}..HEAD", t),
        None => "HEAD".to_string(),
    };
    let out = Command::new("git")
        .args(["log", "--oneline", &range])
        .current_dir(repo_path)
        .output()
        .map_err(|e| ChangelogError::Git(format!("git log 执行失败: {}", e)))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(ChangelogError::Git(if msg.is_empty() {
            "git log 失败".into()
        } else {
            msg
        }));
    }
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err(ChangelogError::NoNewCommits);
    }
    Ok(stdout)
}

/// 获取仓库中最新版本 tag（按语义化版本降序取第一个）。
fn get_latest_tag(repo_path: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["tag", "--list"])
        .current_dir(repo_path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut tags: Vec<&str> = stdout.lines().collect();
    tags.sort_by(|a, b| {
        let a_ver = super::util::parse_tag_semver(a);
        let b_ver = super::util::parse_tag_semver(b);
        b_ver.cmp(&a_ver)
    });
    tags.first().map(|s| s.to_string())
}

/// 调用 LLM 生成 CHANGELOG 条目。
///
/// 通过 quanttide-agent 调用 LLM。如果 LLM 未配置（LLM_API_KEY 为空），
/// 返回提示文本让用户手动发送给 AI（不阻塞发布流程）。
fn llm_changelog(git_log: &str, version: &str) -> Result<String, ChangelogError> {
    let hint = format!(
        "根据以下 git 提交记录，为版本 {} 生成 CHANGELOG 条目。\n\n\
         要求：\n\
         1. 按 Added / Changed / Fixed / Removed 分类\n\
         2. 同类提交合并为概括性条目，不要逐条罗列\n\
         3. 用中文描述\n\
         4. 每类不超过 5 条\n\
         5. 仅输出内容，不要版本头部和日期\n\n\
         提交记录：\n{git_log}",
        version
    );

    let settings = Settings::from_env();
    if settings.llm_api_key.is_empty() {
        return Err(ChangelogError::Llm(format!(
            "LLM 未配置（LLM_API_KEY 未设置）。请将以下文本发送给 AI 生成 CHANGELOG：\n\n{hint}"
        )));
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
             将 git 提交记录按 Added / Changed / Fixed / Removed 分类\
             并合并为概括性条目，用中文描述。不要逐条罗列 commit message。\
             只输出分类后的条目内容，不要输出版本头部（## 行）和日期。",
        ),
        Message::new("user", &hint),
    ];
    let response = llm
        .complete(&messages, CompleteOptions::default())
        .map_err(|e| ChangelogError::Llm(format!("LLM 调用失败: {}", e)))?;

    Ok(response.content.trim().to_string())
}

/// 将生成的 CHANGELOG 条目写入文件。
fn write_changelog(path: &Path, version: &str, content: &str) -> Result<(), ChangelogError> {
    let ver = super::util::normalize_version(version);
    let entry = format!("\n## [{}] - {}\n\n{}\n", ver, today(), content);
    let mut existing = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        "# CHANGELOG\n".to_string()
    };
    if let Some(pos) = existing.find("\n## ") {
        existing.insert_str(pos, &entry);
    } else {
        existing.push_str(&entry);
    }
    std::fs::write(path, &existing)?;
    Ok(())
}

fn today() -> String {
    let zoned = jiff::Zoned::now();
    zoned.strftime("%Y-%m-%d").to_string()
}

/// 如果 CHANGELOG.md 不包含当前版本，则自动生成并写入。
/// 返回 `scope_dir` 相对于 `repo_path` 的 CHANGELOG 路径（供调用方 git add/commit）。
/// 注意：不执行 git add + git commit，调用方需自行提交流程。
pub fn ensure_changelog(repo_path: &Path, scope_dir: &Path, version: &str) -> Result<Option<String>, ChangelogError> {
    let changelog_path = scope_dir.join("CHANGELOG.md");
    if changelog_path.exists() {
        let content = std::fs::read_to_string(&changelog_path)?;
        let ver = super::util::normalize_version(version);
        if content.contains(&format!("[{}]", ver)) || content.contains(&format!("[v{}]", ver)) {
            return Ok(None);
        }
    }
    let git_log = collect_git_log(repo_path)?;
    let changelog_content = llm_changelog(&git_log, version)?;
    write_changelog(&changelog_path, version, &changelog_content)?;
    println!("✓ CHANGELOG.md 已更新（版本 {})", version);

    let rel = changelog_path
        .strip_prefix(repo_path)
        .unwrap_or(&changelog_path)
        .to_str()
        .unwrap_or("CHANGELOG.md")
        .to_string();
    Ok(Some(rel))
}

#[cfg(test)]
mod tests {
    fn git_init(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        cfg.set_str("user.name", "t").unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        let repo = git2::Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("f")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().and_then(|h| h.peel_to_commit()).ok();
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents).unwrap();
    }

    use super::*;

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
    fn test_write_changelog_scope_version() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("CHANGELOG.md");
        write_changelog(&path, "cli/v0.1.0", "### Added\n- feature").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("## [0.1.0]"));
        assert!(!content.contains("## [cli/v0.1.0]"));
    }

    #[test]
    fn test_ensure_changelog_skips_if_exists() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("CHANGELOG.md");
        std::fs::write(&path, "# CHANGELOG\n\n## [0.1.0]\n\ncontent\n").unwrap();
        assert!(ensure_changelog(d.path(), d.path(), "v0.1.0").is_ok());
    }

    #[test]
    fn test_ensure_changelog_no_git_log() {
        let d = tempfile::tempdir().unwrap();
        let result = ensure_changelog(d.path(), d.path(), "v0.1.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_changelog_skips_with_v_prefix() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "# CHANGELOG\n\n## [v0.1.0]\n\ncontent\n",
        )
        .unwrap();
        assert!(ensure_changelog(d.path(), d.path(), "v0.1.0").is_ok());
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

    #[test]
    fn test_latest_tag_semver_v10_greater_than_v9() {
        // 字符串排序 v9.0.0 > v10.0.0，语义化版本排序应反过来
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
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
        assert_eq!(get_latest_tag(d.path()).as_deref(), Some("v10.0.0"));
    }

    #[test]
    fn test_collect_git_log_with_tags_hides_older() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "A");
        std::process::Command::new("git")
            .args(["tag", "v0.1.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        git_commit(d.path(), "B");
        git_commit(d.path(), "C");
        let log = collect_git_log(d.path()).unwrap();
        assert!(log.contains("B"));
        assert!(log.contains("C"));
    }

    #[test]
    fn test_ensure_changelog_repo_path_differs_from_scope_dir() {
        let d = tempfile::tempdir().unwrap();
        let root_dir = d.path().join("repo");
        std::fs::create_dir_all(&root_dir).unwrap();
        git_init(&root_dir);
        git_commit(&root_dir, "init");
        let scope_dir = root_dir.join("sub/scope");
        std::fs::create_dir_all(&scope_dir).unwrap();
        std::fs::write(
            scope_dir.join("CHANGELOG.md"),
            "# CHANGELOG\n\n## [0.1.0]\n\ncontent\n",
        )
        .unwrap();
        assert!(ensure_changelog(&root_dir, &scope_dir, "v0.2.0").is_ok());
        let scoped_content = std::fs::read_to_string(scope_dir.join("CHANGELOG.md")).unwrap();
        assert!(scoped_content.contains("[0.2.0]"));
        assert!(!root_dir.join("CHANGELOG.md").exists());
    }

    #[test]
    fn test_ensure_changelog_appends_new_version() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "# CHANGELOG\n\n## [0.1.0]\n\n### Added\n- init\n",
        )
        .unwrap();
        assert!(ensure_changelog(d.path(), d.path(), "v0.2.0").is_ok());
        let content = std::fs::read_to_string(d.path().join("CHANGELOG.md")).unwrap();
        assert!(content.contains("[0.1.0]"));
        assert!(content.contains("[0.2.0]"));
    }
}
