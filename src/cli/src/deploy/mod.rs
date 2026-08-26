//! deploy 命令：部署生命周期（build → test → release → deploy）。
//!
//! 与 [`crate::release`] 同级，作为全生命周期的一环。核心是 [`init`]：
//! 一键为新仓库/应用生成或升级部署能力（`.github/workflows/deploy-*.yml`
//! + `manifests/terraform/*`），支持 `site` / `studio` / `provider` / `docs` 四种形态。

pub mod audit;
pub mod init;
pub mod status;

pub use audit::audit;
pub use init::{init, GeneratedFile, InitOptions, InitReport};
pub use status::{deploy_status, status};

use std::path::Path;

/// 部署形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DeployKind {
    /// 静态站点（Vite/React web）
    Site,
    /// Flutter Web 应用
    Studio,
    /// 后端服务（provider / API 网关）
    Provider,
    /// 文档站点（myst）
    Docs,
}

impl DeployKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DeployKind::Site => "site",
            DeployKind::Studio => "studio",
            DeployKind::Provider => "provider",
            DeployKind::Docs => "docs",
        }
    }

    /// 当 `--stack` 省略时使用的默认技术栈。
    pub fn default_stack(self) -> DeployStack {
        match self {
            DeployKind::Site => DeployStack::Vite,
            DeployKind::Studio => DeployStack::Flutter,
            DeployKind::Provider => DeployStack::Go,
            DeployKind::Docs => DeployStack::Myst,
        }
    }

    /// 是否为「静态站点 / OSS + CDN」形态（生成 OSS 桶 + CDN + DNS）。
    pub fn is_static_site(self) -> bool {
        matches!(
            self,
            DeployKind::Site | DeployKind::Studio | DeployKind::Docs
        )
    }
}

/// 技术栈。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DeployStack {
    Vite,
    Flutter,
    Go,
    Python,
    Rust,
    Myst,
}

impl DeployStack {
    pub fn as_str(self) -> &'static str {
        match self {
            DeployStack::Vite => "vite",
            DeployStack::Flutter => "flutter",
            DeployStack::Go => "go",
            DeployStack::Python => "python",
            DeployStack::Rust => "rust",
            DeployStack::Myst => "myst",
        }
    }
}

/// 从契约解析 scope，返回 `(部署形态, scope 目录)`。
pub fn resolve_scope(
    repo_path: &Path,
    scope_name: &str,
) -> Result<(DeployKind, String), DeployError> {
    let scopes = crate::contract::load_scopes(repo_path);
    let scope = scopes
        .iter()
        .find(|s| s.name == scope_name)
        .ok_or_else(|| DeployError::ScopeNotFound(scope_name.to_string()))?;
    let dir = scope.dir.clone();
    let lang = scope.language.as_str();
    let kind = DeployKind::from_scope(&scope.name, lang)
        .ok_or_else(|| DeployError::UnsupportedScope(scope.name.clone(), lang.to_string()))?;
    Ok((kind, dir))
}

/// deploy 相关错误。
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("无法解析域名 '{0}'，预期形如 crowd.quanttide.com")]
    InvalidDomain(String),
    #[error("未在契约中找到 scope '{0}'")]
    ScopeNotFound(String),
    #[error("无法从 scope '{0}'（语言 {1}）推导部署形态，可用 --kind 覆盖")]
    UnsupportedScope(String, String),
    #[error("未提供仓库名且无法从 git 远程推断，请用 --repo <name>")]
    RepoMissing,
    #[error("git 远程检测失败: {0}")]
    RepoDetect(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl DeployKind {
    /// 从 scope 名称 + 语言推导部署形态。
    ///
    /// 优先按 scope 名称精确匹配（site/studio/provider/docs），
    /// 无法匹配时按语言回退。
    pub fn from_scope(name: &str, language: &str) -> Option<DeployKind> {
        match name {
            "site" => Some(DeployKind::Site),
            "studio" => Some(DeployKind::Studio),
            "provider" => Some(DeployKind::Provider),
            "docs" => Some(DeployKind::Docs),
            _ => DeployKind::from_language(language),
        }
    }

    fn from_language(language: &str) -> Option<DeployKind> {
        match language {
            "go" => Some(DeployKind::Provider),
            "dart" => Some(DeployKind::Studio),
            "python" => Some(DeployKind::Docs),
            "javascript" | "typescript" | "vite" | "react" => Some(DeployKind::Site),
            "rust" => Some(DeployKind::Site),
            _ => None,
        }
    }
}

/// 从契约语言推导技术栈。
pub fn stack_from_language(language: &str) -> Option<DeployStack> {
    match language {
        "rust" => Some(DeployStack::Rust),
        "go" => Some(DeployStack::Go),
        "dart" => Some(DeployStack::Flutter),
        "python" => Some(DeployStack::Python),
        "javascript" | "typescript" | "vite" | "react" => Some(DeployStack::Vite),
        _ => None,
    }
}

/// 从 git 仓库根推断仓库名（取 toplevel 目录名）。
///
/// 失败时返回 `None`，由调用方决定回退到 `--repo` 参数。
pub fn detect_repo(dir: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let root = String::from_utf8(out.stdout).ok()?;
    Path::new(root.trim())
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
}
