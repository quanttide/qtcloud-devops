use std::path::Path;

use quanttide_agent::{llm::CompleteOptions, Message, Settings, LLM};
use quanttide_devops::source::changelog::{self as lib_changelog, Changelog};

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
    #[error("{0}")]
    Lib(String),
}

impl From<lib_changelog::ChangelogError> for ChangelogError {
    fn from(e: lib_changelog::ChangelogError) -> Self {
        match e {
            lib_changelog::ChangelogError::Io(e) => ChangelogError::Io(e),
            lib_changelog::ChangelogError::Git(s)
            | lib_changelog::ChangelogError::File(s)
            | lib_changelog::ChangelogError::Parse(s) => ChangelogError::Lib(s),
        }
    }
}

fn collect_git_log(repo_path: &Path) -> Result<String, ChangelogError> {
    let tag = get_latest_tag(repo_path);
    let result = lib_changelog::collect_git_log(repo_path, tag.as_deref());
    match result {
        Ok(log) if log.is_empty() => Err(ChangelogError::NoNewCommits),
        Ok(log) => Ok(log),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("没有新的提交") {
                Err(ChangelogError::NoNewCommits)
            } else {
                Err(ChangelogError::from(e))
            }
        }
    }
}

fn get_latest_tag(repo_path: &Path) -> Option<String> {
    // 尝试所有 scope，取最新 tag（raw）
    use quanttide_devops::source::git::tag::{GixTagSource, TagSource};
    let source = GixTagSource::new(repo_path);
    let all = source.all_tags().ok()?;
    let by_semver = |t: &str| quanttide_devops::source::git::tag::parse_semver_tag(t);
    all.iter()
        .max_by(|a, b| by_semver(a).cmp(&by_semver(b)))
        .cloned()
}

fn llm_changelog(git_log: &str, version: &str) -> Result<String, ChangelogError> {
    let hint = lib_changelog::build_changelog_prompt(git_log, version);
    let settings = Settings::from_env();
    if settings.llm_api_key.is_empty() || settings.llm_base_url.is_empty() {
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
        Message::new("system", "你是一个帮助生成 CHANGELOG 的助手。将 git 提交记录按 Added / Changed / Fixed / Removed 分类并合并为概括性条目，用中文描述。不要逐条罗列 commit message。只输出分类后的条目内容，不要输出版本头部（## 行）和日期。"),
        Message::new("user", &hint),
    ];
    let response = llm
        .complete(&messages, CompleteOptions::default())
        .map_err(|e| ChangelogError::Llm(format!("LLM 调用失败: {}", e)))?;
    Ok(response.content.trim().to_string())
}

/// 只读地生成 CHANGELOG 内容（不写盘），供 Plan 阶段使用。
///
/// 返回 `Some(content)` 表示需要追加新条目，`None` 表示版本已存在无需修改。
pub fn generate_changelog_content(
    repo_path: &Path,
    scope_dir: &Path,
    version: &str,
) -> Result<Option<String>, ChangelogError> {
    let changelog_path = scope_dir.join("CHANGELOG.md");
    if changelog_path.exists() {
        if let Ok(cl) = Changelog::from_path(&changelog_path) {
            let ver = crate::contract::normalize_version(version);
            if cl.contains_version(&ver) {
                return Ok(None);
            }
        }
    }
    let git_log = match collect_git_log(repo_path) {
        Ok(log) => log,
        Err(ChangelogError::NoNewCommits) => {
            // 无新提交时，仍允许预发布阶段晋级（如 beta.8 → rc.1）
            return Ok(Some(format!("阶段晋级至 {}", version)));
        }
        Err(e) => return Err(e),
    };
    llm_changelog(&git_log, version).map(Some)
}

/// 将 CHANGELOG 内容写入文件并返回相对路径，仅供 Execute 阶段使用。
pub fn write_changelog_content(
    repo_path: &Path,
    scope_dir: &Path,
    version: &str,
    content: &str,
) -> Result<Option<String>, ChangelogError> {
    let changelog_path = scope_dir.join("CHANGELOG.md");
    lib_changelog::append_entry(&changelog_path, version, content)?;
    let rel = changelog_path
        .strip_prefix(repo_path)
        .unwrap_or(&changelog_path)
        .to_str()
        .unwrap_or("CHANGELOG.md")
        .to_string();
    Ok(Some(rel))
}

/// （已废弃）生成 CHANGELOG 并直接写入文件。请改用 `generate_changelog_content` + `write_changelog_content`。
pub fn ensure_changelog(
    repo_path: &Path,
    scope_dir: &Path,
    version: &str,
) -> Result<Option<String>, ChangelogError> {
    let content = generate_changelog_content(repo_path, scope_dir, version)?;
    match content {
        Some(c) => write_changelog_content(repo_path, scope_dir, version, &c),
        None => Ok(None),
    }
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
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
            .unwrap();
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
    fn test_ensure_changelog_skips_if_exists() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("CHANGELOG.md");
        std::fs::write(&path, "# Changelog\n\n## [0.1.0]\n\ncontent\n").unwrap();
        assert!(ensure_changelog(d.path(), d.path(), "v0.1.0").is_ok());
    }
    #[test]
    fn test_ensure_changelog_no_git_log() {
        let d = tempfile::tempdir().unwrap();
        assert!(ensure_changelog(d.path(), d.path(), "v0.1.0").is_err());
    }
    #[test]
    fn test_ensure_changelog_skips_with_v_prefix() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "# Changelog\n\n## [0.1.0]\n\ncontent\n",
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
            "# Changelog\n\n## [0.1.0]\n\ncontent\n",
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
            "# Changelog\n\n## [0.1.0]\n\n### Added\n- init\n",
        )
        .unwrap();
        assert!(ensure_changelog(d.path(), d.path(), "v0.2.0").is_ok());
        let content = std::fs::read_to_string(d.path().join("CHANGELOG.md")).unwrap();
        assert!(content.contains("[0.1.0]"));
        assert!(content.contains("[0.2.0]"));
    }
    #[test]
    fn test_write_changelog_creates_file() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        assert!(ensure_changelog(d.path(), d.path(), "v0.1.0").is_ok());
        let content = std::fs::read_to_string(d.path().join("CHANGELOG.md")).unwrap();
        assert!(content.contains("## [0.1.0]"));
    }
}
