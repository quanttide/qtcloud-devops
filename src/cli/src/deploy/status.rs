//! deploy status — 当前仓库/应用部署就绪度检查。

use super::{DeployError, DeployKind};
use std::path::Path;

/// 部署就绪度快照。
#[derive(Debug)]
pub struct DeployStatus {
    /// 部署形态。
    pub kind: DeployKind,
    /// 是否存在对应的 deploy workflow。
    pub workflow: bool,
    /// 是否存在 manifests/terraform 目录。
    pub terraform: bool,
    /// 是否存在 SPA 回退改写。
    pub spa_fallback: bool,
    /// 是否实现缓存策略分离。
    pub cache_separation: bool,
    /// 是否配置私有 OSS 回源鉴权。
    pub private_back: bool,
    /// 是否声明所需 org secrets（ALIYUN 凭据）。
    pub secrets: bool,
}

impl DeployStatus {
    /// 就绪度小结字符串。
    pub fn summary(&self) -> String {
        format!(
            "{}: workflow={} terraform={} spa_fallback={} cache_separation={} private_back={} secrets={}",
            self.kind.as_str(),
            check(self.workflow),
            check(self.terraform),
            check(self.spa_fallback),
            check(self.cache_separation),
            check(self.private_back),
            check(self.secrets),
        )
    }
}

fn check(b: bool) -> &'static str {
    if b { "OK" } else { "MISSING" }
}

/// 检查某形态在当前目录下的部署就绪度。
pub fn deploy_status(dir: &Path, kind: DeployKind) -> Result<DeployStatus, DeployError> {
    let workflow = dir
        .join(".github/workflows")
        .join(format!("deploy-{}.yml", kind.as_str()))
        .exists();
    let terraform_dir = dir.join("manifests/terraform");
    let terraform = terraform_dir.exists();
    let cdn = terraform_dir.join("cdn.tf");
    let cdn_content = std::fs::read_to_string(&cdn).unwrap_or_default();
    let spa_fallback = cdn_content.contains("back_to_origin_url_rewrite");
    let private_back = cdn_content.contains("private_oss_auth");
    let workflow_content = workflow_content(dir, kind);
    let cache_separation = workflow_content.contains("max-age=31536000")
        && workflow_content.contains("no-cache");
    let secrets = workflow_content.contains("ALIYUN_ACCESS_KEY_ID")
        || workflow_content.contains("DOCKERHUB_USERNAME");

    Ok(DeployStatus {
        kind,
        workflow,
        terraform,
        spa_fallback,
        cache_separation,
        private_back,
        secrets,
    })
}

fn workflow_content(dir: &Path, kind: DeployKind) -> String {
    let p = dir
        .join(".github/workflows")
        .join(format!("deploy-{}.yml", kind.as_str()));
    std::fs::read_to_string(p).unwrap_or_default()
}

/// 兼容入口：以 `deploy status` 裸调用时，自动探测当前目录是否是静态站点/服务形态。
pub fn status(dir: &Path) -> Vec<DeployStatus> {
    let mut out = Vec::new();
    for kind in [
        DeployKind::Site,
        DeployKind::Studio,
        DeployKind::Provider,
        DeployKind::Docs,
    ] {
        if let Ok(s) = deploy_status(dir, kind) {
            if s.workflow || s.terraform {
                out.push(s);
            }
        }
    }
    out
}
