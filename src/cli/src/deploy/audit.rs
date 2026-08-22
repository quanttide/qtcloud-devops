//! deploy audit — 对现有部署配置做一致性/漂移检测。
//!
//! 当前实现为轻量版：对照平台已上线模板的关键特征（SPA 回退、缓存分离、
//! 私有回源鉴权）检测 `manifests/terraform/cdn.tf` 与 `deploy-*.yml` 是否缺项。

use super::status::DeployStatus;
use super::DeployKind;
use std::path::Path;

/// 一个漂移/缺口项。
#[derive(Debug)]
pub struct AuditItem {
    /// 项目名。
    pub name: String,
    /// 是否达标。
    pub passed: bool,
    /// 详情/建议。
    pub detail: String,
}

/// 对当前目录的部署配置做审计。
pub fn audit(dir: &Path, kind: DeployKind) -> Vec<AuditItem> {
    let status = super::deploy_status(dir, kind).unwrap_or(DeployStatus {
        kind,
        workflow: false,
        terraform: false,
        spa_fallback: false,
        cache_separation: false,
        private_back: false,
        secrets: false,
    });

    vec![
        AuditItem {
            name: format!("deploy-{}.yml 存在", kind.as_str()),
            passed: status.workflow,
            detail: if status.workflow {
                "已存在".into()
            } else {
                "缺失，运行 `deploy init` 生成".into()
            },
        },
        AuditItem {
            name: "terraform manifests 存在".to_string(),
            passed: status.terraform,
            detail: if status.terraform {
                "manifests/terraform/ 已存在".into()
            } else {
                "缺失，运行 `deploy init` 生成".into()
            },
        },
        AuditItem {
            name: "SPA 回退改写".to_string(),
            passed: status.spa_fallback,
            detail: if status.spa_fallback {
                "back_to_origin_url_rewrite 已配置".into()
            } else {
                "缺失，子路由直接访问/刷新会 404".into()
            },
        },
        AuditItem {
            name: "缓存策略分离".to_string(),
            passed: status.cache_separation,
            detail: if status.cache_separation {
                "assets 长缓存 + index.html no-cache 已分离".into()
            } else {
                "缺失，需拆两步上传（产物 max-age=31536000 / 入口 no-cache）".into()
            },
        },
        AuditItem {
            name: "私有 OSS 回源鉴权".to_string(),
            passed: status.private_back,
            detail: if status.private_back {
                "l2_oss_key private_oss_auth=on 已配置".into()
            } else {
                "缺失，RAM 用户无公共读权限时 CDN 无法回源".into()
            },
        },
        AuditItem {
            name: "org secrets 声明".to_string(),
            passed: status.secrets,
            detail: if status.secrets {
                "已引用 ALIYUN/DOCKERHUB secrets".into()
            } else {
                "缺失，需配置 ALIYUN_ACCESS_KEY_ID / _SECRET 等 org secrets".into()
            },
        },
    ]
}
