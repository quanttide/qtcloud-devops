//! deploy 命令：部署生命周期（build → test → release → deploy）。
//!
//! 与 [`crate::release`] 同级，作为全生命周期的一环。核心是 [`init`]：
//! 一键为新仓库/应用生成或升级部署能力（`.github/workflows/deploy-*.yml`
//! + `manifests/terraform/*`），支持 `site` / `studio` / `provider` / `docs` 四种形态。

pub mod audit;
pub mod init;
pub mod status;

pub use audit::audit;
pub use init::{init, InitOptions, InitReport, GeneratedFile};
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
        matches!(self, DeployKind::Site | DeployKind::Studio | DeployKind::Docs)
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

/// deploy 相关错误。
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("无法解析域名 '{0}'，预期形如 crowd.quanttide.com")]
    InvalidDomain(String),
    #[error("未提供仓库名且无法从 git 远程推断，请用 --repo <name>")]
    RepoMissing,
    #[error("git 远程检测失败: {0}")]
    RepoDetect(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
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
