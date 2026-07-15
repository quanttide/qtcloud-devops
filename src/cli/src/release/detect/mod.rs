mod inference;
mod scope_util;
mod tag_util;

use std::path::Path;

pub(crate) use tag_util::get_latest_tag_for_scope;

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
    super::util::git::git(args, repo_path).map_err(DetectError::Git)
}

/// 主入口。
pub fn detect_version(repo_path: &Path) -> Result<DetectResult, DetectError> {
    let root = git_output(&["rev-parse", "--show-toplevel"], repo_path)
        .map_err(|_| DetectError::Other(format!("不在 git 仓库中: {:?}", repo_path)))?;
    let root = Path::new(&root);

    let project_type = scope_util::detect_project_type(root);
    println!("📌 项目类型: {}", project_type);

    let scope = scope_util::detect_single_scope(root)?;
    println!("📌 scope: {:?}", scope);

    let latest_tag = tag_util::get_latest_tag_for_scope(root, scope.as_deref());
    let (has_tag, major, minor, patch, pre_stage, pre_num) = match latest_tag {
        Some(ref tag) => {
            let (_, ver_str) = tag_util::parse_tag(tag);
            let (ma, mi, pa, st, nu) = tag_util::parse_version(ver_str)?;
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

    if has_tag {
        let tag = latest_tag.as_ref().unwrap();
        let tag_rev = git_output(&["rev-parse", &format!("refs/tags/{}", tag)], root)
            .map_err(|_| DetectError::Other("找不到标签引用".into()))?;
        let head_rev = git_output(&["rev-parse", "HEAD"], root)
            .map_err(|_| DetectError::Other("找不到 HEAD".into()))?;
        if tag_rev == head_rev {
            return Err(DetectError::Other("上次标签后没有新提交".into()));
        }
    }

    let range = latest_tag
        .as_ref()
        .map_or("HEAD".to_string(), |t| format!("{}..HEAD", t));
    let log_output = git_output(&["log", "--oneline", &range], root).unwrap_or_default();
    let commits = inference::parse_commit_messages(&log_output);

    println!("📝 提交数: {}", commits.len());
    for c in &commits {
        println!("   • {}", c);
    }

    if commits.is_empty() {
        return Err(DetectError::Other("没有提交记录".into()));
    }

    let llm_tag = latest_tag.as_deref().unwrap_or("(新项目，无版本标签)");
    let decision = inference::llm_decide(
        &commits,
        llm_tag,
        project_type,
        scope.as_deref().unwrap_or("(root)"),
    )?;

    println!("🧠 LLM 决策: {}", decision.reason);

    let new_version = inference::build_version_from_decision(
        has_tag,
        &tag_util::VersionParts {
            major,
            minor,
            patch,
            pre_stage: pre_stage.clone(),
            pre_num,
        },
        &decision,
    )?;

    let version = tag_util::apply_scope_prefix(scope.as_deref(), &new_version);

    println!("\n🔮 建议版本: {}", version);
    Ok(DetectResult { version })
}
