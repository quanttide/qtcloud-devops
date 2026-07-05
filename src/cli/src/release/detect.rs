/// 版本号自动检测 — 为 release publish 提供自动版本推断。
///
/// 移植自实验室 detect 原型，复用已有的 git2 和 quanttide-agent 依赖。
use quanttide_agent::llm::{CompleteOptions, LLM};
use quanttide_agent::message::Message;
use quanttide_agent::Settings;
use std::collections::HashMap;
use std::path::Path;

/// 检测结果。
pub struct DetectResult {
    /// 推断出的完整版本号（含 scope 前缀）。
    pub version: String,
    /// scope 名称。
    pub scope: Option<String>,
    /// 项目类型：code / docs
    pub project_type: String,
}

/// 主入口：检测并推断下一个版本号。
///
/// 打印详细输出（项目类型、tag、提交、LLM 决策），返回推断结果。
pub fn detect_version(repo_path: &Path) -> Result<DetectResult, String> {
    let repo = git2::Repository::discover(repo_path).map_err(|e| format!("打开仓库失败: {}", e))?;

    let project_type = detect_project_type(&repo);
    println!("📌 项目类型: {}", project_type);

    let scope = detect_single_scope(&repo)?;
    println!("📌 scope: {:?}", scope);

    // ── 读最新 tag ────────────────────────────────────────────────
    let latest_tag = get_latest_tag_for_scope(&repo, scope.as_deref());
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
    let head_oid = repo
        .head()
        .and_then(|h| h.target().ok_or_else(|| git2::Error::from_str("")))
        .map_err(|_| "找不到 HEAD")?;

    let mut revwalk = repo.revwalk().map_err(|_| "创建 revwalk 失败")?;
    revwalk.push(head_oid).ok();
    if has_tag {
        let tag = latest_tag.as_ref().unwrap();
        let tag_oid = repo
            .find_reference(&format!("refs/tags/{}", tag))
            .and_then(|r| r.target().ok_or_else(|| git2::Error::from_str("")))
            .map_err(|_| "找不到标签引用")?;
        if head_oid == tag_oid {
            return Err("上次标签后没有新提交".into());
        }
        revwalk.hide(tag_oid).ok();
    }

    let mut commits: Vec<String> = Vec::new();
    for oid in revwalk {
        let oid = match oid {
            Ok(o) => o,
            Err(_) => continue,
        };
        if let Ok(commit) = repo.find_commit(oid) {
            let msg = commit.summary().unwrap_or("").to_string();
            commits.push(msg);
        }
    }

    println!("📝 提交数: {}", commits.len());
    for c in &commits {
        println!("   • {}", c);
    }

    if commits.is_empty() {
        return Err("没有提交记录".into());
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

    let new_version = if !has_tag {
        // 新项目：首个版本固定为 v0.1.0，预发布阶段由 LLM 决定
        match decision.prerelease.as_deref() {
            Some(pr) => format!("v0.1.0-{}.1", pr),
            None => "v0.1.0".to_string(),
        }
    } else if decision.action == "skip" {
        return Err("无需发版".into());
    } else if decision.action == "human" {
        return Err(format!("需要人类判断: {}", decision.reason));
    } else {
        let increment = decision.increment.as_deref().unwrap_or("patch");
        build_version(
            major,
            minor,
            patch,
            pre_stage.as_deref(),
            pre_num,
            increment,
            decision.prerelease.as_deref(),
        )
    };

    let version = match scope {
        Some(ref s) if !s.is_empty() && s != "(root)" => format!("{}/{}", s, new_version),
        _ => new_version.clone(),
    };

    println!("\n🔮 建议版本: {}", version);
    Ok(DetectResult {
        version,
        scope,
        project_type: project_type.to_string(),
    })
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
) -> Result<LlmDecision, String> {
    let settings = Settings::from_env();
    if settings.llm_api_key.is_empty() {
        return Ok(fallback_heuristic(commits));
    }

    let llm = LLM::new(
        &settings.llm_model,
        &settings.llm_base_url,
        &settings.llm_api_key,
    );

    let commits_text = commits
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {}", i + 1, c))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
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
- **绝大多数变更都是 patch**。新增文档、更新内容、格式规范化、目录结构调整都是日常工作。
- minor 仅限全新内容品类上线的程度（例如从零搭建了一整套新手册），极少发生。
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
    );

    let messages = vec![
        Message::new(
            "system",
            "你是一个严格的版本号推断工具。只输出 JSON，不要额外内容。",
        ),
        Message::new("user", &prompt),
    ];

    let options = CompleteOptions {
        response_format: Some(serde_json::json!({"type": "json_object"})),
        ..Default::default()
    };

    let resp = llm
        .complete(&messages, options)
        .map_err(|e| format!("LLM 调用失败: {}", e.0))?;

    let decision: LlmDecision = serde_json::from_str(&resp.content)
        .map_err(|e| format!("LLM 输出解析失败: {} — 原始输出: {}", e, resp.content))?;

    Ok(decision)
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
    major: u32,
    minor: u32,
    patch: u32,
    pre_stage: Option<&str>,
    pre_num: Option<u32>,
    increment: &str,
    prerelease: Option<&str>,
) -> String {
    if let Some(stage) = pre_stage {
        let next = pre_num.unwrap_or(0) + 1;
        return format!("v{}.{}.{}-{}.{}", major, minor, patch, stage, next);
    }

    match (increment, prerelease) {
        ("minor", Some(pr)) => format!("v{}.{}.{}-{}.1", major, minor + 1, 0, pr),
        ("minor", None) => format!("v{}.{}.{}", major, minor + 1, 0),
        _ => format!("v{}.{}.{}", major, minor, patch + 1),
    }
}

// ═════════════════════════════════════════════════════════════════════
// 项目类型检测
// ═════════════════════════════════════════════════════════════════════

/// 检测项目类型。
fn detect_project_type(repo: &git2::Repository) -> &'static str {
    let workdir = match repo.workdir() {
        Some(d) => d,
        None => return "unknown",
    };
    let indicators = [
        workdir.join("src").is_dir(),
        workdir.join("Cargo.toml").exists(),
        workdir.join("package.json").exists(),
        workdir.join("pyproject.toml").exists(),
        workdir.join("setup.py").exists(),
        workdir.join("go.mod").exists(),
        workdir.join("packages").is_dir(),
        workdir.join("apps").is_dir(),
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
fn detect_single_scope(repo: &git2::Repository) -> Result<Option<String>, String> {
    let scopes = load_contract_scopes(repo.workdir().unwrap_or(Path::new(".")));
    let changed_paths = get_changed_paths_since_last_tag(repo)?;

    let mut hits: HashMap<String, usize> = HashMap::new();
    for path in &changed_paths {
        for (name, dir) in &scopes {
            if path.starts_with(dir.trim_start_matches('/')) || path.contains(dir) {
                *hits.entry(name.clone()).or_insert(0) += 1;
            }
        }
    }

    let best = hits.iter().max_by_key(|(_, c)| *c);
    if let Some((name, _)) = best {
        return Ok(Some(name.clone()));
    }

    // 回退：从已有 tag 收集 scope
    let all_tags = collect_tags_with_scope(repo);
    let scoped: Vec<&String> = all_tags.keys().filter(|k| *k != "(root)").collect();
    if scoped.len() == 1 {
        return Ok(Some(scoped[0].clone()));
    }
    if scoped.len() > 1 {
        let names: Vec<&str> = scoped.iter().map(|s| s.as_str()).collect();
        return Err(format!("多个 scope 有变更: {:?}，请用 -v 指定", names));
    }

    Ok(None) // (root)
}

fn load_contract_scopes(repo_root: &Path) -> HashMap<String, String> {
    let paths = [
        repo_root.join(".quanttide/devops/contract.yaml"),
        repo_root.join("contract.yaml"),
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(cfg) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(scopes) = cfg.get("scopes").and_then(|s| s.as_mapping()) {
                    let mut map = HashMap::new();
                    for (k, v) in scopes {
                        let name = k.as_str().unwrap_or("").to_string();
                        let dir = v
                            .get("dir")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();
                        map.insert(name, dir);
                    }
                    return map;
                }
            }
        }
    }
    HashMap::new()
}

fn get_changed_paths_since_last_tag(repo: &git2::Repository) -> Result<Vec<String>, String> {
    let head_oid = repo
        .head()
        .and_then(|h| h.target().ok_or_else(|| git2::Error::from_str("")))
        .map_err(|_| "找不到 HEAD")?;

    let tree = repo
        .find_commit(head_oid)
        .and_then(|c| c.tree())
        .map_err(|_| "找不到 HEAD tree")?;

    let tags = collect_tags_with_scope(repo);
    let latest_tag = tags
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .find_map(|(_, v)| v.first())
        .or_else(|| tags.get("(root)").and_then(|v| v.first()));

    let base_tree = match latest_tag {
        Some(tag) => {
            let tag_oid = repo
                .find_reference(&format!("refs/tags/{}", tag))
                .and_then(|r| r.target().ok_or_else(|| git2::Error::from_str("")))
                .ok()
                .and_then(|oid| repo.find_commit(oid).ok())
                .and_then(|c| c.tree().ok());
            tag_oid
        }
        None => None,
    };

    let diff = repo
        .diff_tree_to_tree(base_tree.as_ref(), Some(&tree), None)
        .map_err(|_| "diff 失败".to_string())?;

    let mut paths: Vec<String> = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(f) = delta.new_file().path() {
                paths.push(f.to_string_lossy().to_string());
            }
            true
        },
        None,
        None,
        None,
    )
    .ok();

    Ok(paths)
}

// ═════════════════════════════════════════════════════════════════════
// tag 处理
// ═════════════════════════════════════════════════════════════════════

fn get_latest_tag_for_scope(repo: &git2::Repository, scope: Option<&str>) -> Option<String> {
    let all = collect_tags_with_scope(repo);
    let scope_key = scope.unwrap_or("(root)");
    all.get(scope_key).and_then(|tags| tags.first().cloned())
}

fn collect_tags_with_scope(repo: &git2::Repository) -> HashMap<String, Vec<String>> {
    let tag_names = match repo.tag_names(None) {
        Ok(t) => t,
        Err(_) => return HashMap::new(),
    };
    let mut groups: HashMap<String, Vec<((u32, u32, u32, u32, u32), String)>> = HashMap::new();
    for tag in tag_names.iter().flatten() {
        let (scope, ver_str) = parse_tag(tag);
        let scope_name = scope.unwrap_or_else(|| "(root)".to_string());
        if let Ok((major, minor, patch, _, pre_num)) = parse_version(ver_str) {
            let pre_ord = pre_num.unwrap_or(0);
            let stage_ord = if ver_str.contains("-alpha") {
                1
            } else if ver_str.contains("-beta") {
                2
            } else if ver_str.contains("-rc") {
                3
            } else {
                0
            };
            let ord = (major, minor, patch, stage_ord, pre_ord);
            groups
                .entry(scope_name)
                .or_default()
                .push((ord, tag.to_string()));
        }
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

fn parse_version(s: &str) -> Result<(u32, u32, u32, Option<String>, Option<u32>), String> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let (ver_part, pre_part) = s.split_once('-').unwrap_or((s, ""));
    let parts: Vec<&str> = ver_part.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("版本号格式错误: {}，需要 X.Y.Z", s));
    }
    let major = parts[0].parse().map_err(|_| "major 不是数字".to_string())?;
    let minor = parts[1].parse().map_err(|_| "minor 不是数字".to_string())?;
    let patch: u32 = parts[2].parse().map_err(|_| "patch 不是数字".to_string())?;
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
        assert_eq!(build_version(0, 8, 4, None, None, "patch", None), "v0.8.5");
    }

    #[test]
    fn test_build_version_minor_rc() {
        assert_eq!(
            build_version(0, 8, 4, None, None, "minor", Some("rc")),
            "v0.9.0-rc.1"
        );
    }

    #[test]
    fn test_build_version_prerelease_increment() {
        assert_eq!(
            build_version(0, 9, 0, Some("rc"), Some(1), "patch", None),
            "v0.9.0-rc.2"
        );
    }

    #[test]
    fn test_build_version_minor_formal() {
        assert_eq!(build_version(0, 8, 4, None, None, "minor", None), "v0.9.0");
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
}
