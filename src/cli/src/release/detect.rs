use std::collections::HashMap;
use std::path::{Path, PathBuf};

use quanttide_agent::Settings;

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("git 操作失败: {0}")]
    Git(String),
    #[error("LLM 调用失败: {0}")]
    Llm(String),
    #[error("版本号格式错误: {0}")]
    Version(String),
    #[error("{0}")]
    Other(String),
}

impl From<String> for DetectError {
    fn from(s: String) -> Self {
        DetectError::Other(s)
    }
}

/// 检测结果。
pub struct DetectResult {
    pub version: String,
}

/// 在 repo_path 下执行 git 命令，输出到 stdout（去尾空白）。
fn git_output(args: &[&str], repo_path: &Path) -> Result<String, DetectError> {
    crate::source::git::git(args, repo_path).map_err(DetectError::Git)
}

/// 主入口。
pub fn detect_version(repo_path: &Path) -> Result<DetectResult, DetectError> {
    let root = resolve_git_root(repo_path)?;
    let project_type = detect_project_type(&root);
    println!("📌 项目类型: {}", project_type);

    let scope = detect_single_scope(&root)?;
    println!("📌 scope: {:?}", scope);

    let latest_tag = crate::source::tag::get_latest_tag_for_scope(&root, scope.as_deref());
    let (has_tag, major, minor, patch, pre_stage, pre_num) = parse_and_print_tag(latest_tag.as_ref());

    if has_tag && !has_new_commits_since_tag(&root, latest_tag.as_ref().unwrap()) {
        return Err(DetectError::Other("上次标签后没有新提交".into()));
    }

    let commits = collect_commit_messages(&root, latest_tag.as_ref());
    if commits.is_empty() {
        return Err(DetectError::Other("没有提交记录".into()));
    }

    let llm_tag = latest_tag.as_deref().unwrap_or("(新项目，无版本标签)");
    let decision = llm_decide(&commits, llm_tag, project_type, scope.as_deref().unwrap_or("(root)"))?;
    println!("🧠 LLM 决策: {}", decision.reason);

    let new_version = build_version_from_decision(has_tag, &crate::source::tag::VersionParts {
        major, minor, patch, pre_stage: pre_stage.clone(), pre_num,
    }, &decision)?;

    let version = crate::source::tag::apply_scope_prefix(scope.as_deref(), &new_version);
    println!("\n🔮 建议版本: {}", version);
    Ok(DetectResult { version })
}

fn resolve_git_root(repo_path: &Path) -> Result<PathBuf, DetectError> {
    let root = git_output(&["rev-parse", "--show-toplevel"], repo_path)
        .map_err(|_| DetectError::Other(format!("不在 git 仓库中: {:?}", repo_path)))?;
    Ok(PathBuf::from(root.trim()))
}

fn parse_and_print_tag(latest_tag: Option<&String>) -> (bool, u32, u32, u32, Option<String>, Option<u32>) {
    match latest_tag {
        Some(tag) => {
            let (_, ver_str) = crate::source::tag::parse_tag(tag);
            let (ma, mi, pa, st, nu): (u32, u32, u32, Option<String>, Option<u32>) = crate::source::tag::parse_version(ver_str).unwrap_or((0, 0, 0, None, None));
            println!("📦 最新标签: {}", tag);
            println!("   v{}.{}.{}", ma, mi, pa);
            if let Some(ref stage) = st {
                println!("   预发布: {}.{}", stage, nu.unwrap_or(0));
            }
            (true, ma, mi, pa, st, nu)
        }
        None => {
            println!("📦 没有版本标签（新项目）");
            (false, 0, 1, 0, None, None)
        }
    }
}

fn has_new_commits_since_tag(root: &Path, tag: &str) -> bool {
    let tag_rev = git_output(&["rev-parse", &format!("refs/tags/{}", tag)], root).ok();
    let head_rev = git_output(&["rev-parse", "HEAD"], root).ok();
    tag_rev.zip(head_rev).map(|(t, h)| t != h).unwrap_or(false)
}

fn collect_commit_messages(root: &Path, latest_tag: Option<&String>) -> Vec<String> {
    let range = latest_tag
        .as_ref()
        .map_or("HEAD".to_string(), |t| format!("{}..HEAD", t));
    let log_output = git_output(&["log", "--oneline", &range], root).unwrap_or_default();
    let commits = crate::source::git::parse_commit_messages(&log_output);
    println!("📝 提交数: {}", commits.len());
    for c in &commits {
        println!("   • {}", c);
    }
    commits
}

// ═══════════════════════════════════════════════════════════════════════
// scope 检测
// ═══════════════════════════════════════════════════════════════════════

fn detect_project_type(root: &Path) -> &'static str {
    let indicators = [
        root.join("src").is_dir(),
        root.join("Cargo.toml").exists(),
        root.join("package.json").exists(),
        root.join("pyproject.toml").exists(),
        root.join("setup.py").exists(),
        root.join("go.mod").exists(),
        root.join("packages").is_dir(),
        root.join("apps").is_dir(),
    ];
    if indicators.iter().any(|&x| x) {
        "code"
    } else {
        "docs"
    }
}

fn detect_single_scope(root: &Path) -> Result<Option<String>, DetectError> {
    let scopes = crate::contract::load_scopes(root);
    let changed_paths = get_changed_paths_since_last_tag(root)?;

    let mut hits: HashMap<String, usize> = HashMap::new();
    for path in &changed_paths {
        for scope in &scopes {
            if path.starts_with(scope.dir.trim_start_matches('/')) || path.contains(&scope.dir) {
                *hits.entry(scope.name.clone()).or_insert(0) += 1;
            }
        }
    }

    let best = hits.iter().max_by_key(|(_, c)| *c);
    if let Some((name, _)) = best {
        return Ok(Some(name.clone()));
    }

    let all_tags = crate::source::tag::collect_tags_with_scope(root);
    let scoped: Vec<&String> = all_tags.keys().filter(|k| *k != "(root)").collect();
    if scoped.len() == 1 {
        return Ok(Some(scoped[0].clone()));
    }
    if scoped.len() > 1 {
        let names: Vec<&str> = scoped.iter().map(|s| s.as_str()).collect();
        return Err(DetectError::Other(format!(
            "多个 scope 有变更: {:?}，请用 -v 指定",
            names
        )));
    }

    Ok(None)
}

fn get_changed_paths_since_last_tag(root: &Path) -> Result<Vec<String>, DetectError> {
    let tags = crate::source::tag::collect_tags_with_scope(root);
    let latest_tag = tags
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .find_map(|(_, v)| v.first())
        .or_else(|| tags.get("(root)").and_then(|v| v.first()));

    let range = match latest_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => return Ok(vec![]),
    };

    let output = crate::source::git::git(&["diff", "--name-only", &range], root).unwrap_or_default();
    Ok(output.lines().map(|s| s.to_string()).collect())
}

// ═══════════════════════════════════════════════════════════════════════
// 版本号推断
// ═══════════════════════════════════════════════════════════════════════

fn build_version_from_decision(
    has_tag: bool,
    parts: &crate::source::tag::VersionParts,
    decision: &LlmDecision,
) -> Result<String, DetectError> {
    if !has_tag {
        return match decision.prerelease.as_deref() {
            Some(pr) => Ok(format!("v0.1.0-{}.1", pr)),
            None => Ok("v0.1.0".to_string()),
        };
    }
    if decision.action == "skip" {
        return Err(DetectError::Other("无需发版".into()));
    }
    if decision.action == "human" {
        return Err(DetectError::Other(format!(
            "需要人类判断: {}",
            decision.reason
        )));
    }
    let increment = decision.increment.as_deref().unwrap_or("patch");
    Ok(crate::source::tag::build_version(
        parts,
        increment,
        decision.prerelease.as_deref(),
    ))
}

fn llm_decide(
    commits: &[String],
    latest_tag: &str,
    project_type: &str,
    scope: &str,
) -> Result<LlmDecision, DetectError> {
    let settings = Settings::from_env();
    if settings.llm_api_key.is_empty() || settings.llm_base_url.is_empty() {
        return Ok(fallback_heuristic(commits));
    }

    let prompt = build_version_prompt(commits, latest_tag, project_type, scope);
    call_llm_decision(&prompt, &settings)
}

fn build_version_prompt(
    commits: &[String],
    latest_tag: &str,
    project_type: &str,
    scope: &str,
) -> String {
    let commits_text = commits
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {}", i + 1, c))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"你是一个版本号推断专家。根据以下信息，决定下一个版本号策略。

## 约束
- 不做 major bump（breaking change 交给人类）
- 仅 chore/typo/CI 配置 → skip
- `docs:` 是内容变更（文档项目的交付物），不是非逻辑改动
- patch 级别修复 → 直发正式版
- minor 级别新功能 → 代码项目走预发布（rc），文档项目直发正式
- 大版本早期未完成功能 → alpha
- 功能基本完成 → beta
- 功能冻结只修 bug → rc
- 已在预发布系列 → 同阶段递增序号（除非有理由晋级下一阶段）

### 如何判断 minor vs patch

**代码项目：**
- `feat:` → minor（追加新能力）
- `fix: / refactor: / test:` → patch（修问题）

**内容/文档项目：**
- **绝大多数变更都是 patch**。
- minor 仅限全新内容品类上线的程度，极少发生。
- 不确定时就 patch。

## 当前版本
项目类型: {project_type}
最新 tag: {tag}
scope: {scope}

## 提交记录（tag→HEAD）
{commits}

## 输出格式（仅 JSON）
{{"action": "release"|"skip"|"human", "increment": "minor"|"patch"|null, "prerelease": "alpha"|"beta"|"rc"|null, "reason": "判断理由"}}
"#,
        tag = latest_tag,
        scope = scope,
        project_type = project_type,
        commits = commits_text,
    )
}

fn call_llm_decision(prompt: &str, settings: &Settings) -> Result<LlmDecision, DetectError> {
    use quanttide_agent::{llm::CompleteOptions, Message, LLM};
    let llm = LLM::new(
        &settings.llm_model,
        &settings.llm_base_url,
        &settings.llm_api_key,
    );
    let messages = vec![
        Message::new(
            "system",
            "你是一个严格的版本号推断工具。只输出 JSON，不要额外内容。",
        ),
        Message::new("user", prompt),
    ];
    let options = CompleteOptions {
        response_format: Some(serde_json::json!({"type": "json_object"})),
        ..Default::default()
    };
    let resp = llm
        .complete(&messages, options)
        .map_err(|e| DetectError::Llm(format!("LLM 调用失败: {}", e.0)))?;
    serde_json::from_str(&resp.content).map_err(|e| {
        DetectError::Llm(format!(
            "LLM 输出解析失败: {} — 原始输出: {}",
            e, resp.content
        ))
    })
}

fn fallback_heuristic(commits: &[String]) -> LlmDecision {
    let mut has_feat = false;
    let mut has_breaking = false;
    let mut has_logic_change = false;

    for msg in commits {
        let lower = msg.to_lowercase();
        if lower.contains("breaking") || (msg.contains('!') && lower.starts_with("feat")) {
            has_breaking = true;
            has_logic_change = true;
        } else if lower.starts_with("feat") || msg.contains("Added") {
            has_feat = true;
            has_logic_change = true;
        } else if lower.starts_with("fix")
            || lower.starts_with("docs")
            || lower.starts_with("refactor")
            || lower.starts_with("test")
            || msg.contains("Fixed")
            || msg.contains("Changed")
        {
            has_logic_change = true;
        }
    }

    build_decision_from_flags(has_feat, has_breaking, has_logic_change)
}

fn build_decision_from_flags(
    has_feat: bool,
    has_breaking: bool,
    has_logic_change: bool,
) -> LlmDecision {
    if !has_logic_change {
        return LlmDecision {
            action: "skip".into(),
            increment: None,
            prerelease: None,
            reason: "仅有 chore/typo/CI 改动，无需发版".into(),
        };
    }

    if has_breaking {
        return LlmDecision {
            action: "human".into(),
            increment: None,
            prerelease: None,
            reason: "包含 breaking change，请人类指定 major 版本号".into(),
        };
    }

    let (increment, reason) = if has_feat {
        ("minor", "包含 feat，minor 增量直发正式")
    } else {
        ("patch", "包含 docs/fix/refactor，patch 增量直发正式")
    };

    LlmDecision {
        action: "release".into(),
        increment: Some(increment.into()),
        prerelease: None,
        reason: reason.into(),
    }
}

#[derive(serde::Deserialize)]
struct LlmDecision {
    pub action: String,
    pub increment: Option<String>,
    pub prerelease: Option<String>,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── build_version_from_decision ────────────────────────────────

    #[test]
    fn test_build_version_from_decision_no_tag() {
        let d = LlmDecision {
            action: "release".into(),
            increment: Some("minor".into()),
            prerelease: None,
            reason: "".into(),
        };
        let v = build_version_from_decision(
            false,
            &crate::source::tag::VersionParts {
                major: 0,
                minor: 0,
                patch: 0,
                pre_stage: None,
                pre_num: None,
            },
            &d,
        )
        .unwrap();
        assert_eq!(v, "v0.1.0");
    }

    #[test]
    fn test_build_version_from_decision_no_tag_prerelease() {
        let d = LlmDecision {
            action: "release".into(),
            increment: Some("minor".into()),
            prerelease: Some("alpha".into()),
            reason: "".into(),
        };
        let v = build_version_from_decision(
            false,
            &crate::source::tag::VersionParts {
                major: 0,
                minor: 0,
                patch: 0,
                pre_stage: None,
                pre_num: None,
            },
            &d,
        )
        .unwrap();
        assert_eq!(v, "v0.1.0-alpha.1");
    }

    #[test]
    fn test_build_version_from_decision_skip() {
        let d = LlmDecision {
            action: "skip".into(),
            increment: None,
            prerelease: None,
            reason: "".into(),
        };
        assert!(build_version_from_decision(
            true,
            &crate::source::tag::VersionParts {
                major: 1,
                minor: 0,
                patch: 0,
                pre_stage: None,
                pre_num: None
            },
            &d
        )
        .is_err());
    }

    #[test]
    fn test_build_version_from_decision_human() {
        let d = LlmDecision {
            action: "human".into(),
            increment: None,
            prerelease: None,
            reason: "breaking change".into(),
        };
        assert!(build_version_from_decision(
            true,
            &crate::source::tag::VersionParts {
                major: 1,
                minor: 0,
                patch: 0,
                pre_stage: None,
                pre_num: None
            },
            &d
        )
        .is_err());
    }

    #[test]
    fn test_build_version_from_decision_patch() {
        let d = LlmDecision {
            action: "release".into(),
            increment: Some("patch".into()),
            prerelease: None,
            reason: "".into(),
        };
        let v = build_version_from_decision(
            true,
            &crate::source::tag::VersionParts {
                major: 0,
                minor: 8,
                patch: 4,
                pre_stage: None,
                pre_num: None,
            },
            &d,
        )
        .unwrap();
        assert_eq!(v, "v0.8.5");
    }

    // ── fallback_heuristic ─────────────────────────────────────────

    #[test]
    fn test_fallback_heuristic_feat() {
        let d = fallback_heuristic(&["feat: add command".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("minor"));
    }

    #[test]
    fn test_fallback_heuristic_fix() {
        let d = fallback_heuristic(&["fix: resolve crash".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("patch"));
    }

    #[test]
    fn test_fallback_heuristic_docs() {
        let d = fallback_heuristic(&["docs: update readme".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("patch"));
    }

    #[test]
    fn test_fallback_heuristic_skip() {
        let d = fallback_heuristic(&["chore: bump version".into()]);
        assert_eq!(d.action, "skip");
    }

    #[test]
    fn test_fallback_heuristic_breaking() {
        let d = fallback_heuristic(&["feat!: breaking".into()]);
        assert_eq!(d.action, "human");
    }

    #[test]
    fn test_fallback_heuristic_refactor() {
        let d = fallback_heuristic(&["refactor: extract method".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("patch"));
    }

    #[test]
    fn test_fallback_heuristic_test_commit() {
        let d = fallback_heuristic(&["test: add coverage".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("patch"));
    }

    #[test]
    fn test_fallback_heuristic_added_commits() {
        let d = fallback_heuristic(&["Added new feature".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("minor"));
    }

    #[test]
    fn test_fallback_heuristic_fixed_commits() {
        let d = fallback_heuristic(&["Fixed crash on startup".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("patch"));
    }

    #[test]
    fn test_fallback_heuristic_changed_commits() {
        let d = fallback_heuristic(&["Changed behavior of X".into()]);
        assert_eq!(d.action, "release");
        assert_eq!(d.increment.as_deref(), Some("patch"));
    }

    // ── detect_project_type ────────────────────────────────────────

    #[test]
    fn test_detect_project_type_code_with_src() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("src")).unwrap();
        assert_eq!(detect_project_type(d.path()), "code");
    }

    #[test]
    fn test_detect_project_type_code_with_cargo() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_project_type(d.path()), "code");
    }

    #[test]
    fn test_detect_project_type_docs() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(d.path()), "docs");
    }

    #[test]
    fn test_detect_project_type_no_workdir() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(d.path()), "docs");
    }

    fn git_init_detect(path: &std::path::Path) {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output()
            .unwrap();
        std::fs::write(path.join(".gitkeep"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_commit_file(repo_path: &std::path::Path, path: &str, content: &str) {
        std::fs::write(repo_path.join(path), content).unwrap();
        std::process::Command::new("git")
            .args(["add", path])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", &format!("update {path}")])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    fn git_tag(repo_path: &std::path::Path, tag: &str) {
        std::process::Command::new("git")
            .args(["tag", tag])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_get_changed_paths_no_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_commit_file(d.path(), "new.txt", "hello");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.is_empty(), "无 tag 时无法 diff，应返回空");
    }

    #[test]
    fn test_get_changed_paths_after_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_commit_file(d.path(), "initial.txt", "initial");
        git_tag(d.path(), "v1.0.0");
        git_commit_file(d.path(), "added.txt", "added");
        git_commit_file(d.path(), "modified.txt", "modified");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.contains(&"added.txt".to_string()));
        assert!(paths.contains(&"modified.txt".to_string()));
        assert!(!paths.contains(&"initial.txt".to_string()), "tag 前的文件不应出现");
    }

    #[test]
    fn test_get_changed_paths_no_new_commits() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v1.0.0");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.is_empty(), "无新提交应返回空");
    }

    #[test]
    fn test_detect_single_scope_no_changes() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        let scope = detect_single_scope(d.path()).unwrap();
        assert_eq!(scope, None, "无 tag 无 contract 应返回 None");
    }

    #[test]
    fn test_detect_single_scope_fallback_to_tags() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        std::fs::create_dir_all(d.path().join("packages/cli")).unwrap();
        git_commit_file(d.path(), "packages/cli/readme.md", "cli");
        git_tag(d.path(), "cli/v0.1.0");
        git_commit_file(d.path(), "readme.md", "root");
        let scope = detect_single_scope(d.path()).unwrap();
        assert_eq!(scope.as_deref(), Some("cli"), "唯一有 tag 的 scope");
    }

    #[test]
    fn test_detect_single_scope_root_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_commit_file(d.path(), "file.txt", "content");
        git_tag(d.path(), "v1.0.0");
        let scope = detect_single_scope(d.path()).unwrap();
        assert_eq!(scope, None, "只有 root tag 应返回 None");
    }
}
