/// 版本号自动检测 — 为 release publish 提供自动版本推断。
use quanttide_agent::llm::{CompleteOptions, LLM};
use quanttide_agent::message::Message;
use quanttide_agent::Settings;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

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

/// 在 repo_path 下执行 git 命令，输出到 stdout（去尾空白）。
fn git_output(args: &[&str], repo_path: &Path) -> Result<String, DetectError> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| DetectError::Git(format!("git 无法执行: {}", e)))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(DetectError::Git(if msg.is_empty() {
            "git 命令失败".into()
        } else {
            msg
        }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 检测结果。
pub struct DetectResult {
    pub version: String,
}

/// 主入口。
pub fn detect_version(repo_path: &Path) -> Result<DetectResult, DetectError> {
    // 确保是在 git 仓库中
    let root = git_output(&["rev-parse", "--show-toplevel"], repo_path)
        .map_err(|_| DetectError::Other(format!("不在 git 仓库中: {:?}", repo_path)))?;
    let root = Path::new(&root);

    let project_type = detect_project_type(root);
    println!("📌 项目类型: {}", project_type);

    let scope = detect_single_scope(root)?;
    println!("📌 scope: {:?}", scope);

    // ── 读最新 tag ────────────────────────────────────────────────
    let latest_tag = get_latest_tag_for_scope(root, scope.as_deref());
    let (has_tag, major, minor, patch, pre_stage, pre_num) = match latest_tag {
        Some(ref tag) => {
            let (_, ver_str) = parse_tag(tag);
            let (ma, mi, pa, st, nu) = parse_version(ver_str)?;
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
    };

    // ── 扫描 tag→HEAD 提交 ────────────────────────────────────────
    if has_tag {
        let tag = latest_tag.as_ref().unwrap();
        // 检查 tag 是否就是 HEAD
        let tag_rev = git_output(&["rev-parse", &format!("refs/tags/{}", tag)], root)
            .map_err(|_| DetectError::Other("找不到标签引用".into()))?;
        let head_rev = git_output(&["rev-parse", "HEAD"], root).map_err(|_| DetectError::Other("找不到 HEAD".into()))?;
        if tag_rev == head_rev {
            return Err(DetectError::Other("上次标签后没有新提交".into()));
        }
    }

    let range = latest_tag
        .as_ref()
        .map_or("HEAD".to_string(), |t| format!("{}..HEAD", t));
    let log_output = git_output(&["log", "--oneline", &range], root).unwrap_or_default();
    let commits = parse_commit_messages(&log_output);

    println!("📝 提交数: {}", commits.len());
    for c in &commits {
        println!("   • {}", c);
    }

    if commits.is_empty() {
        return Err(DetectError::Other("没有提交记录".into()));
    }

    // ── LLM 推断版本（回退到启发式规则）───────────────────────────
    let llm_tag = latest_tag.as_deref().unwrap_or("(新项目，无版本标签)");
    let decision = llm_decide(
        &commits,
        llm_tag,
        &project_type,
        scope.as_deref().unwrap_or("(root)"),
    )?;

    println!("🧠 LLM 决策: {}", decision.reason);

    let new_version = build_version_from_decision(
        has_tag,
        &VersionParts { major, minor, patch, pre_stage: pre_stage.clone(), pre_num },
        &decision,
    )?;

    let version = apply_scope_prefix(scope.as_deref(), &new_version);

    println!("\n🔮 建议版本: {}", version);
    Ok(DetectResult { version })
}

/// 从 git log --oneline 输出解析提交消息列表。
fn parse_commit_messages(log_output: &str) -> Vec<String> {
    log_output
        .lines()
        .map(|l| {
            if l.len() > 8 {
                l[7..].trim().to_string()
            } else {
                l.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// 版本号组成部分，用于 `build_version_from_decision` / `build_version` 替代长参数列表。
struct VersionParts {
    major: u32,
    minor: u32,
    patch: u32,
    pre_stage: Option<String>,
    pre_num: Option<u32>,
}

/// 根据 LLM 决策构建版本号（不含 scope 前缀）。
fn build_version_from_decision(
    has_tag: bool,
    parts: &VersionParts,
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
        return Err(DetectError::Other(format!("需要人类判断: {}", decision.reason)));
    }
    let increment = decision.increment.as_deref().unwrap_or("patch");
    Ok(build_version(parts, increment, decision.prerelease.as_deref()))
}

/// 给版本号添加 scope 前缀。
fn apply_scope_prefix(scope: Option<&str>, version: &str) -> String {
    match scope {
        Some(s) if !s.is_empty() && s != "(root)" => format!("{}/{}", s, version),
        _ => version.to_string(),
    }
}

/// LLM 决策输出。
#[derive(serde::Deserialize)]
struct LlmDecision {
    action: String,             // "release" | "skip" | "human"
    increment: Option<String>,  // "minor" | "patch" | null
    prerelease: Option<String>, // "alpha" | "beta" | "rc" | null
    reason: String,
}

/// 调用 LLM 决定版本策略。未配置 LLM 时回退到启发式规则。
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

fn build_version_prompt(commits: &[String], latest_tag: &str, project_type: &str, scope: &str) -> String {
    let commits_text = commits.iter().enumerate()
        .map(|(i, c)| format!("{}. {}", i + 1, c))
        .collect::<Vec<_>>().join("\n");
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
        tag = latest_tag, scope = scope, project_type = project_type, commits = commits_text,
    )
}

fn call_llm_decision(prompt: &str, settings: &Settings) -> Result<LlmDecision, DetectError> {
    use quanttide_agent::{llm::CompleteOptions, Message, LLM};
    let llm = LLM::new(&settings.llm_model, &settings.llm_base_url, &settings.llm_api_key);
    let messages = vec![
        Message::new("system", "你是一个严格的版本号推断工具。只输出 JSON，不要额外内容。"),
        Message::new("user", prompt),
    ];
    let options = CompleteOptions {
        response_format: Some(serde_json::json!({"type": "json_object"})),
        ..Default::default()
    };
    let resp = llm.complete(&messages, options)
        .map_err(|e| DetectError::Llm(format!("LLM 调用失败: {}", e.0)))?;
    serde_json::from_str(&resp.content)
        .map_err(|e| DetectError::Llm(format!("LLM 输出解析失败: {} — 原始输出: {}", e, resp.content)))
}

/// 启发式回退规则。
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

/// 根据启发式标记构建决策。
fn build_decision_from_flags(has_feat: bool, has_breaking: bool, has_logic_change: bool) -> LlmDecision {
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

/// 根据决策构建版本字符串（不含 scope 前缀）。
fn build_version(
    parts: &VersionParts,
    increment: &str,
    prerelease: Option<&str>,
) -> String {
    if let Some(stage) = &parts.pre_stage {
        let next = parts.pre_num.unwrap_or(0) + 1;
        return format!("v{}.{}.{}-{}.{}", parts.major, parts.minor, parts.patch, stage, next);
    }

    match (increment, prerelease) {
        ("minor", Some(pr)) => format!("v{}.{}.{}-{}.1", parts.major, parts.minor + 1, 0, pr),
        ("minor", None) => format!("v{}.{}.{}", parts.major, parts.minor + 1, 0),
        _ => format!("v{}.{}.{}", parts.major, parts.minor, parts.patch + 1),
    }
}

// ═════════════════════════════════════════════════════════════════════
// 项目类型检测
// ═════════════════════════════════════════════════════════════════════

/// 检测项目类型。
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

// ═════════════════════════════════════════════════════════════════════
// scope 检测
// ═════════════════════════════════════════════════════════════════════

/// 从 contract.yaml + 变化文件推断最佳 scope。
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

    // 回退：从已有 tag 收集 scope
    let all_tags = collect_tags_with_scope(root);
    let scoped: Vec<&String> = all_tags.keys().filter(|k| *k != "(root)").collect();
    if scoped.len() == 1 {
        return Ok(Some(scoped[0].clone()));
    }
    if scoped.len() > 1 {
        let names: Vec<&str> = scoped.iter().map(|s| s.as_str()).collect();
        return Err(DetectError::Other(format!("多个 scope 有变更: {:?}，请用 -v 指定", names)));
    }

    Ok(None) // (root)
}

fn get_changed_paths_since_last_tag(root: &Path) -> Result<Vec<String>, DetectError> {
    // 取最新 tag
    let tags = collect_tags_with_scope(root);
    let latest_tag = tags
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .find_map(|(_, v)| v.first())
        .or_else(|| tags.get("(root)").and_then(|v| v.first()));

    let range = match latest_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => return Ok(vec![]),
    };

    let output = git_output(&["diff", "--name-only", &range], root).unwrap_or_default();
    Ok(output.lines().map(|s| s.to_string()).collect())
}

// ═════════════════════════════════════════════════════════════════════
// tag 处理
// ═════════════════════════════════════════════════════════════════════

pub(crate) fn get_latest_tag_for_scope(root: &Path, scope: Option<&str>) -> Option<String> {
    let scope_name = scope.unwrap_or("");
    quanttide_devops::source::git_tag::latest_tag(root, scope_name).ok().flatten()
}

fn collect_tags_with_scope(root: &Path) -> HashMap<String, Vec<String>> {
    use quanttide_devops::source::git_tag::{GixTagSource, TagSource, parse_semver_tag};
    let source = GixTagSource::new(root);
    let all = match source.all_tags() { Ok(t) => t, Err(_) => return HashMap::new() };
    let mut groups: HashMap<String, Vec<(Option<semver::Version>, String)>> = HashMap::new();
    for tag in &all {
        let (scope, _) = tag.split_once('/').unwrap_or(("", tag));
        let scope_name = if scope.is_empty() { "(root)".to_string() } else { scope.to_string() };
        groups.entry(scope_name).or_default().push((parse_semver_tag(tag), tag.clone()));
    }
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for (scope, mut entries) in groups {
        entries.sort_by(|a, b| b.0.cmp(&a.0));
        result.insert(scope, entries.into_iter().map(|(_, t)| t).collect());
    }
    result
}

fn parse_tag(tag: &str) -> (Option<String>, &str) {
    if let Some((scope, ver)) = tag.split_once('/') {
        (Some(scope.to_string()), ver)
    } else {
        (None, tag)
    }
}

fn parse_version(s: &str) -> Result<(u32, u32, u32, Option<String>, Option<u32>), DetectError> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let (ver_part, pre_part) = s.split_once('-').unwrap_or((s, ""));
    let parts: Vec<&str> = ver_part.split('.').collect();
    if parts.len() != 3 {
        return Err(DetectError::Version(format!("版本号格式错误: {}，需要 X.Y.Z", s)));
    }
    let major = parts[0].parse().map_err(|_| DetectError::Version("major 不是数字".into()))?;
    let minor = parts[1].parse().map_err(|_| DetectError::Version("minor 不是数字".into()))?;
    let patch: u32 = parts[2].parse().map_err(|_| DetectError::Version("patch 不是数字".into()))?;
    let (pre_stage, pre_num) = if pre_part.is_empty() {
        (None, None)
    } else {
        let sp: Vec<&str> = pre_part.split('.').collect();
        let stage = sp.first().map(|s| s.to_string());
        let num = sp.get(1).and_then(|s| s.parse().ok());
        (stage, num)
    };
    Ok((major, minor, patch, pre_stage, pre_num))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tag_scoped() {
        assert_eq!(parse_tag("cli/v0.8.4"), (Some("cli".into()), "v0.8.4"));
    }

    #[test]
    fn test_parse_tag_root() {
        assert_eq!(parse_tag("v0.1.0"), (None, "v0.1.0"));
    }

    #[test]
    fn test_parse_version_formal() {
        let (ma, mi, pa, st, nu) = parse_version("0.8.4").unwrap();
        assert_eq!((ma, mi, pa), (0, 8, 4));
        assert!(st.is_none());
        assert!(nu.is_none());
    }

    #[test]
    fn test_parse_version_prerelease() {
        let (ma, mi, pa, st, nu) = parse_version("0.9.0-rc.1").unwrap();
        assert_eq!((ma, mi, pa), (0, 9, 0));
        assert_eq!(st.as_deref(), Some("rc"));
        assert_eq!(nu, Some(1));
    }

    #[test]
    fn test_parse_version_with_v_prefix() {
        let (ma, mi, pa, _, _) = parse_version("v0.8.4").unwrap();
        assert_eq!((ma, mi, pa), (0, 8, 4));
    }

    #[test]
    fn test_parse_version_bad_format() {
        assert!(parse_version("abc").is_err());
        assert!(parse_version("0.1").is_err());
    }

    #[test]
    fn test_build_version_patch() {
        assert_eq!(build_version(&VersionParts { major: 0, minor: 8, patch: 4, pre_stage: None, pre_num: None }, "patch", None), "v0.8.5");
    }

    #[test]
    fn test_build_version_minor_rc() {
        assert_eq!(
            build_version(&VersionParts { major: 0, minor: 8, patch: 4, pre_stage: None, pre_num: None }, "minor", Some("rc")),
            "v0.9.0-rc.1"
        );
    }

    #[test]
    fn test_build_version_prerelease_increment() {
        assert_eq!(
            build_version(&VersionParts { major: 0, minor: 9, patch: 0, pre_stage: Some("rc".into()), pre_num: Some(1) }, "patch", None),
            "v0.9.0-rc.2"
        );
    }

    #[test]
    fn test_build_version_minor_formal() {
        assert_eq!(build_version(&VersionParts { major: 0, minor: 8, patch: 4, pre_stage: None, pre_num: None }, "minor", None), "v0.9.0");
    }

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

    // ── 更多 parse_tag 边缘 ─────────────────────────────────────

    #[test]
    fn test_parse_tag_multiple_slashes() {
        assert_eq!(
            parse_tag("scope/v0.1.0-rc.1"),
            (Some("scope".into()), "v0.1.0-rc.1")
        );
    }

    #[test]
    fn test_parse_tag_empty() {
        assert_eq!(parse_tag(""), (None, ""));
    }

    // ── 更多 parse_version 边缘 ─────────────────────────────────

    #[test]
    fn test_parse_version_alpha() {
        let (ma, mi, pa, st, nu) = parse_version("1.0.0-alpha.1").unwrap();
        assert_eq!((ma, mi, pa), (1, 0, 0));
        assert_eq!(st.as_deref(), Some("alpha"));
        assert_eq!(nu, Some(1));
    }

    #[test]
    fn test_parse_version_beta() {
        let (ma, mi, pa, st, nu) = parse_version("0.5.0-beta.2").unwrap();
        assert_eq!((ma, mi, pa), (0, 5, 0));
        assert_eq!(st.as_deref(), Some("beta"));
        assert_eq!(nu, Some(2));
    }

    #[test]
    fn test_parse_version_prerelease_no_number() {
        let (ma, mi, pa, st, nu) = parse_version("1.2.3-rc").unwrap();
        assert_eq!((ma, mi, pa), (1, 2, 3));
        assert_eq!(st.as_deref(), Some("rc"));
        assert_eq!(nu, None);
    }

    #[test]
    fn test_parse_version_non_numeric_parts() {
        assert!(parse_version("a.b.c").is_err());
        assert!(parse_version("1.x.3").is_err());
    }

    // ── 更多 build_version ─────────────────────────────────────

    #[test]
    fn test_build_version_patch_with_same_stage() {
        assert_eq!(
            build_version(&VersionParts { major: 1, minor: 0, patch: 0, pre_stage: Some("beta".into()), pre_num: Some(3) }, "patch", None),
            "v1.0.0-beta.4"
        );
    }

    #[test]
    fn test_build_version_minor_with_alpha() {
        assert_eq!(
            build_version(&VersionParts { major: 0, minor: 1, patch: 0, pre_stage: None, pre_num: None }, "minor", Some("alpha")),
            "v0.2.0-alpha.1"
        );
    }

    #[test]
    fn test_build_version_no_prerelease_info() {
        // prerelease 为 None 但 increment 不是 patch → 直发正式
        assert_eq!(build_version(&VersionParts { major: 1, minor: 0, patch: 0, pre_stage: None, pre_num: None }, "patch", None), "v1.0.1");
    }

    // ── fallback_heuristic 更多模式 ───────────────────────────

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
        // "Added" 开头（传统提交风格）应识别为 feat
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

    // ── parse_commit_messages ─────────────────────────────────

    #[test]
    fn test_parse_commit_messages_typical() {
        let msgs = parse_commit_messages("abc1234 feat: add foo\ndef5678 fix: bar\n");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0], "feat: add foo");
        assert_eq!(msgs[1], "fix: bar");
    }

    #[test]
    fn test_parse_commit_messages_empty() {
        assert!(parse_commit_messages("").is_empty());
    }

    #[test]
    fn test_parse_commit_messages_short_line() {
        let msgs = parse_commit_messages("abc1234\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], "abc1234");
    }

    // ── build_version_from_decision ────────────────────────────

    #[test]
    fn test_build_version_from_decision_no_tag() {
        let d = LlmDecision {
            action: "release".into(),
            increment: Some("minor".into()),
            prerelease: None,
            reason: "".into(),
        };
        let v = build_version_from_decision(false, &VersionParts { major: 0, minor: 0, patch: 0, pre_stage: None, pre_num: None }, &d).unwrap();
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
        let v = build_version_from_decision(false, &VersionParts { major: 0, minor: 0, patch: 0, pre_stage: None, pre_num: None }, &d).unwrap();
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
        assert!(build_version_from_decision(true, &VersionParts { major: 1, minor: 0, patch: 0, pre_stage: None, pre_num: None }, &d).is_err());
    }

    #[test]
    fn test_build_version_from_decision_human() {
        let d = LlmDecision {
            action: "human".into(),
            increment: None,
            prerelease: None,
            reason: "breaking change".into(),
        };
        assert!(build_version_from_decision(true, &VersionParts { major: 1, minor: 0, patch: 0, pre_stage: None, pre_num: None }, &d).is_err());
    }

    #[test]
    fn test_build_version_from_decision_patch() {
        let d = LlmDecision {
            action: "release".into(),
            increment: Some("patch".into()),
            prerelease: None,
            reason: "".into(),
        };
        let v = build_version_from_decision(true, &VersionParts { major: 0, minor: 8, patch: 4, pre_stage: None, pre_num: None }, &d).unwrap();
        assert_eq!(v, "v0.8.5");
    }

    // ── apply_scope_prefix ────────────────────────────────────

    #[test]
    fn test_apply_scope_prefix_with_scope() {
        assert_eq!(apply_scope_prefix(Some("cli"), "v0.1.0"), "cli/v0.1.0");
    }

    #[test]
    fn test_apply_scope_prefix_root() {
        assert_eq!(apply_scope_prefix(Some("(root)"), "v0.1.0"), "v0.1.0");
    }

    #[test]
    fn test_apply_scope_prefix_none() {
        assert_eq!(apply_scope_prefix(None, "v0.1.0"), "v0.1.0");
    }

    #[test]
    fn test_apply_scope_prefix_empty() {
        assert_eq!(apply_scope_prefix(Some(""), "v0.1.0"), "v0.1.0");
    }

    // ── detect_project_type ───────────────────────────────────

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
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-m",
                "init",
            ])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_detect_project_type_code_with_src() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        std::fs::create_dir(d.path().join("src")).unwrap();
        // detect_project_type 直接检查目录，不需要 git2
        assert_eq!(detect_project_type(d.path()), "code");
    }

    #[test]
    fn test_detect_project_type_code_with_cargo() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        std::fs::write(d.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_project_type(d.path()), "code");
    }

    #[test]
    fn test_detect_project_type_docs() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        assert_eq!(detect_project_type(d.path()), "docs");
    }

    #[test]
    fn test_detect_project_type_no_workdir() {
        // 非 git 目录直接检查路径
        let d = tempfile::tempdir().unwrap();
        // 空目录没有代码指示物 → docs
        assert_eq!(detect_project_type(d.path()), "docs");
    }

    // ── git tag 辅助函数 ──────────────────────────────────────

    fn git_tag(repo_path: &std::path::Path, tag: &str) {
        std::process::Command::new("git")
            .args(["tag", tag])
            .current_dir(repo_path)
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
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-m",
                &format!("update {path}"),
            ])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    // ── collect_tags_with_scope ───────────────────────────────

    #[test]
    fn test_collect_tags_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        let tags = collect_tags_with_scope(d.path());
        assert!(tags.is_empty());
    }

    #[test]
    fn test_collect_tags_root_only() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v1.0.0");
        git_tag(d.path(), "v1.1.0");
        let tags = collect_tags_with_scope(d.path());
        assert_eq!(tags.len(), 1);
        assert!(tags.contains_key("(root)"));
        assert_eq!(tags["(root)"], vec!["v1.1.0", "v1.0.0"]);
    }

    #[test]
    fn test_collect_tags_scoped_ordered() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "cli/v0.2.0");
        git_tag(d.path(), "cli/v0.3.0");
        git_tag(d.path(), "cli/v0.1.0");
        let tags = collect_tags_with_scope(d.path());
        assert_eq!(tags.len(), 1);
        assert!(tags.contains_key("cli"));
        assert_eq!(tags["cli"], vec!["cli/v0.3.0", "cli/v0.2.0", "cli/v0.1.0"]);
    }

    #[test]
    fn test_collect_tags_multi_scope_with_prerelease() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v0.5.0");
        git_tag(d.path(), "cli/v0.2.0-rc.1");
        git_tag(d.path(), "cli/v0.2.0");
        git_tag(d.path(), "sdk/v0.1.0-alpha.1");
        git_tag(d.path(), "sdk/v0.1.0-beta.1");
        git_tag(d.path(), "sdk/v0.1.0");
        let tags = collect_tags_with_scope(d.path());
        assert_eq!(tags.len(), 3);
        // semver 排序: 正式版 > rc > beta > alpha
        assert_eq!(tags["cli"][0], "cli/v0.2.0", "正式版应排首位");
        assert_eq!(tags["cli"][1], "cli/v0.2.0-rc.1");
        // sdk: 降序排列: 正式 > beta > alpha
        assert!(tags["sdk"][0].contains("v0.1.0"), "正式版应排首位");
        assert!(tags["sdk"][1].contains("beta"), "beta 应在 alpha 之前");
        assert!(tags["sdk"][2].contains("alpha"), "alpha 应排最后");
        assert_eq!(tags["(root)"][0], "v0.5.0");
    }

    // ── get_latest_tag_for_scope ───────────────────────────────

    #[test]
    fn test_get_latest_tag_root_scope() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v1.0.0");
        git_tag(d.path(), "v2.0.0");
        assert_eq!(
            get_latest_tag_for_scope(d.path(), None).as_deref(),
            Some("v2.0.0")
        );
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("(root)")).as_deref(),
            Some("v2.0.0")
        );
    }

    #[test]
    fn test_get_latest_tag_scoped() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "cli/v0.1.0");
        git_tag(d.path(), "cli/v0.2.0");
        git_tag(d.path(), "sdk/v0.5.0");
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("cli")).as_deref(),
            Some("cli/v0.2.0")
        );
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("sdk")).as_deref(),
            Some("sdk/v0.5.0")
        );
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("nosuch")).as_deref(),
            None
        );
    }

    // ── get_changed_paths_since_last_tag ───────────────────────

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
        // 创建第一个文件并打 tag
        git_commit_file(d.path(), "initial.txt", "initial");
        git_tag(d.path(), "v1.0.0");
        // 后续变更
        git_commit_file(d.path(), "added.txt", "added");
        git_commit_file(d.path(), "modified.txt", "modified");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.contains(&"added.txt".to_string()));
        assert!(paths.contains(&"modified.txt".to_string()));
        assert!(
            !paths.contains(&"initial.txt".to_string()),
            "tag 前的文件不应出现"
        );
    }

    #[test]
    fn test_get_changed_paths_no_new_commits() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v1.0.0");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.is_empty(), "无新提交应返回空");
    }

    // ── detect_single_scope ───────────────────────────────────

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
        // 只有一个 scope 有 tag
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
