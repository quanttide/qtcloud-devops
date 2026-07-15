use quanttide_agent::Settings;

use super::DetectError;
use super::tag_util::{build_version, VersionParts};

pub(super) fn parse_commit_messages(log_output: &str) -> Vec<String> {
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

pub(super) fn build_version_from_decision(
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
        return Err(DetectError::Other(format!(
            "需要人类判断: {}",
            decision.reason
        )));
    }
    let increment = decision.increment.as_deref().unwrap_or("patch");
    Ok(build_version(
        parts,
        increment,
        decision.prerelease.as_deref(),
    ))
}

pub(super) fn llm_decide(
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

pub(super) fn fallback_heuristic(commits: &[String]) -> LlmDecision {
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
pub(super) struct LlmDecision {
    pub action: String,
    pub increment: Option<String>,
    pub prerelease: Option<String>,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_commit_messages ─────────────────────────────────────

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
            &VersionParts {
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
            &VersionParts {
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
            &VersionParts {
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
            &VersionParts {
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
            &VersionParts {
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
}
