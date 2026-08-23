//! deploy init — 一键为新仓库/应用生成或升级部署能力。
//!
//! 就地写入 `.github/workflows/deploy-{kind}.yml` + `manifests/terraform/*`，
//! 从平台已上线版本（如 qtcrowd）收敛为可复用参数化模板，自动处理
//! SPA 回退、缓存策略分离、私有 OSS 回源鉴权、SSL 证书占位等已知坑。

use super::{detect_repo, DeployError, DeployKind, DeployStack};
use std::path::{Path, PathBuf};

/// `deploy init` 的输入参数。
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// 站点域名，如 `crowd.quanttide.com`。
    pub domain: String,
    /// 契约 scope 名称（如 `studio` / `provider`）。
    pub scope: String,
    /// OSS 桶名，省略时按 `{repo}-{kind}` 推导。
    pub bucket: Option<String>,
    /// 仓库名，省略时从 git 远程推断。
    pub repo: Option<String>,
    /// 覆盖已存在的文件。
    pub force: bool,
    /// 只预览要生成的文件与内容，不落盘。
    pub dry_run: bool,
    /// 仓库根路径（用于加载契约与推导 scope 目录）。
    pub repo_path: PathBuf,
}

/// 单个生成的部署文件。
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// 相对基准目录的路径。
    pub path: PathBuf,
    /// 文件内容。
    pub content: String,
    /// 该文件在磁盘上已存在。
    pub existed: bool,
}

/// `deploy init` 的结果报告。
#[derive(Debug)]
pub struct InitReport {
    /// 生成/预览的文件清单。
    pub files: Vec<GeneratedFile>,
    /// 推导出的 OSS 桶名。
    pub bucket: String,
    /// 推导出的 CDN 域名。
    pub cdn_domain: String,
    /// 使用的技术栈。
    pub stack: DeployStack,
    /// 推导出的部署形态。
    pub kind: DeployKind,
    /// scope 目录（仓库根相对）。
    pub scope_dir: String,
}

#[derive(Debug)]
struct DomainParts {
    rr: String,
    cdn_domain: String,
}

/// 运行 `deploy init`。
pub fn init(opts: &InitOptions) -> Result<InitReport, DeployError> {
    let scopes = crate::contract::load_scopes(&opts.repo_path);
    let scope = scopes
        .iter()
        .find(|s| s.name == opts.scope)
        .ok_or_else(|| DeployError::ScopeNotFound(opts.scope.clone()))?;
    let scope_dir = scope.dir.clone();
    let language = scope.language.as_str();
    let kind = DeployKind::from_scope(&scope.name, language)
        .ok_or_else(|| DeployError::UnsupportedScope(scope.name.clone(), language.to_string()))?;

    let stack = super::stack_from_language(language).unwrap_or_else(|| kind.default_stack());

    let bucket = match &opts.bucket {
        Some(b) => b.clone(),
        None => {
            let repo = opts
                .repo
                .clone()
                .or_else(|| detect_repo(&opts.repo_path))
                .ok_or(DeployError::RepoMissing)?;
            format!("{}-{}", repo, kind.as_str())
        }
    };

    let domain = parse_domain(&opts.domain)?;
    let cdn_domain = if kind.is_static_site() {
        domain.cdn_domain.clone()
    } else {
        String::new()
    };

    // 从桶名反推仓库名（bucket 格式 {repo}-{kind}）
    let suffix = format!("-{}", kind.as_str());
    let repo_name = bucket.strip_suffix(&suffix).unwrap_or(&bucket).to_string();

    let files = render_files(kind, stack, &bucket, &repo_name, &scope_dir, &domain);

    // 输出基准目录：scope 目录
    let base = opts.repo_path.join(&scope_dir);
    if !opts.dry_run {
        for f in &files {
            write_if_needed(&base, f, opts.force)?;
        }
    }

    Ok(InitReport {
        files,
        bucket,
        cdn_domain,
        stack,
        kind,
        scope_dir,
    })
}

/// 解析域名，返回 `(rr, cdn_domain)`。
///
/// - `crowd.quanttide.com` → rr=`crowd`, cdn_domain=`crowd.quanttide.com`
/// - `quanttide.com`（apex）→ rr=`@`, cdn_domain=`quanttide.com`
fn parse_domain(domain: &str) -> Result<DomainParts, DeployError> {
    let domain = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return Err(DeployError::InvalidDomain(domain.to_string()));
    }
    let rr = if labels.len() == 2 { "@".to_string() } else { labels[0].to_string() };
    let cdn_domain = domain.to_string();
    Ok(DomainParts { rr, cdn_domain })
}

/// 为指定形态生成全部部署文件（相对路径 + 内容）。
fn render_files(kind: DeployKind, stack: DeployStack, bucket: &str, repo_name: &str, scope_dir: &str, d: &DomainParts) -> Vec<GeneratedFile> {
    let mut files = Vec::new();

    if kind.is_static_site() {
        let workflow = PathBuf::from(format!(".github/workflows/deploy-{}.yml", kind.as_str()));
        files.push(GeneratedFile {
            path: workflow,
            content: render_workflow(kind, stack, bucket, repo_name, scope_dir, d),
            existed: false,
        });

        let kind_str = kind.as_str();
        let tf: [(String, String); 8] = [
            (String::from("cdn.tf"), render_cdn_tf(kind, bucket, d)),
            (format!("{}-bucket.tf", kind_str), render_bucket_tf(kind, bucket)),
            (String::from("locals.tf"), render_locals_tf(kind, bucket)),
            (String::from("outputs.tf"), render_outputs_tf(kind)),
            (String::from("platform.tf"), render_platform_tf()),
            (String::from("providers.tf"), render_providers_tf()),
            (String::from("variables.tf"), render_variables_tf(bucket)),
            (String::from("versions.tf"), render_versions_tf()),
        ];
        for (name, content) in tf {
            files.push(GeneratedFile {
                path: PathBuf::from("manifests/terraform").join(name),
                content,
                existed: false,
            });
        }
    } else {
        // provider / 非静态形态：生成 Docker 镜像发布 workflow，无 OSS 桶/CDN。
        let workflow = PathBuf::from(format!(".github/workflows/deploy-{}.yml", kind.as_str()));
        files.push(GeneratedFile {
            path: workflow,
            content: render_docker_workflow(kind),
            existed: false,
        });
    }

    files
}

fn write_if_needed(base: &Path, f: &GeneratedFile, force: bool) -> Result<(), DeployError> {
    let full = base.join(&f.path);
    if full.exists() && !force {
        // 已存在且未强制：跳过（由调用方提示）
        return Ok(());
    }
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full, &f.content)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Workflow 渲染
// ═══════════════════════════════════════════════════════════════════

fn render_workflow(kind: DeployKind, stack: DeployStack, bucket: &str, repo_name: &str, scope_dir: &str, d: &DomainParts) -> String {
    let name = match kind {
        DeployKind::Site => "Deploy Site",
        DeployKind::Studio => "Deploy Studio",
        DeployKind::Docs => "Deploy Docs",
        _ => "Deploy",
    };
    let (build_cmd, out_rel) = build_parts(stack);
    // build 目录与上传目录均相对仓库根；空 scope_dir 视为仓库根自身。
    let build_dir = if scope_dir.is_empty() { ".".to_string() } else { scope_dir.to_string() };
    let upload_dir = if out_rel.is_empty() {
        build_dir.clone()
    } else if scope_dir.is_empty() {
        out_rel.to_string()
    } else {
        format!("{}/{}", scope_dir, out_rel)
    };
    let title = format!("# 部署 {}：推送 {}/* tag 时，应用基础设施（Terraform），构建产物上传 OSS，刷新 CDN。\n# 所需 GitHub secrets（org 级）：\n#   - ALIYUN_ACCESS_KEY_ID / ALIYUN_ACCESS_KEY_SECRET：RAM 用户 AccessKey（terraform + OSS 上传 + CDN 刷新）\n", kind.as_str(), kind.as_str());

    format!(
        r#"{title}
name: {name}

on:
  push:
    tags:
      - '{kind}/**'

concurrency:
  group: deploy-{kind}
  cancel-in-progress: false

env:
  OSS_BUCKET: {bucket}
  OSS_ENDPOINT: oss-cn-hangzhou.aliyuncs.com
  CDN_DOMAIN: {cdn_domain}

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22

      - name: Setup terraform
        uses: hashicorp/setup-terraform@v3

      - name: Apply infrastructure (Terraform)
        working-directory: manifests/terraform
        run: |
          terraform init \
            -backend-config="bucket=${{{{ secrets.TF_STATE_BUCKET || 'quanttide-terraform-state' }}}}" \
            -backend-config="key={repo_name}/{kind}.tfstate" \
            -backend-config="region=cn-hangzhou"
          terraform apply -input=false -auto-approve \
            -var region=cn-hangzhou \
            -var environment=prod \
            -var oss_bucket_name={bucket} \
            -var image=placeholder \
            -target=alicloud_oss_bucket.{kind_resource} \
            -target=alicloud_cdn_domain_new.{kind_resource} \
            -target=alicloud_cdn_domain_config.{kind_resource}_private_back \
            -target=alicloud_cdn_domain_config.{kind_resource}_spa_fallback \
            -target=alicloud_cdn_domain_config.{kind_resource}_https_force \
            -target=alicloud_alidns_record.{kind_resource}
        env:
          ALICLOUD_ACCESS_KEY: ${{{{ secrets.ALIYUN_ACCESS_KEY_ID }}}}
          ALICLOUD_SECRET_KEY: ${{{{ secrets.ALIYUN_ACCESS_KEY_SECRET }}}}

      - name: Build site
        working-directory: {build_dir}
        run: |
          {build_cmd}

      - name: Setup ossutil
        uses: manyuanrong/setup-ossutil@v3.0
        with:
          endpoint: ${{{{ env.OSS_ENDPOINT }}}}
          access-key-id: ${{{{ secrets.ALIYUN_ACCESS_KEY_ID }}}}
          access-key-secret: ${{{{ secrets.ALIYUN_ACCESS_KEY_SECRET }}}}
      # 缓存策略分离：assets/* 长缓存；入口 index.html no-cache
      - name: Upload to OSS（产物长缓存）
        run: ossutil cp {upload_dir}/ oss://${{{{ env.OSS_BUCKET }}}}/ -r -f --meta=Cache-Control:public,max-age=31536000
      - name: Upload to OSS（入口文件 no-cache）
        run: |
          for f in index.html; do
            ossutil cp "{upload_dir}/${{{{ f }}}}" "oss://${{{{ env.OSS_BUCKET }}}}/${{{{ f }}}}" -f --meta=Cache-Control:no-cache
          done

      - name: Refresh CDN
        run: |
          pip install aliyun-python-sdk-cdn
          python -c "
          from aliyunsdkcore.client import AcsClient
          from aliyunsdkcdn.request.v20180510.RefreshObjectCachesRequest import RefreshObjectCachesRequest
          import os
          cli = AcsClient(os.environ['ALIYUN_ACCESS_KEY_ID'], os.environ['ALIYUN_ACCESS_KEY_SECRET'], 'cn-hangzhou')
          req = RefreshObjectCachesRequest()
          req.set_ObjectPath('https://${{{{ env.CDN_DOMAIN }}}}/')
          req.set_ObjectType('Directory')
          cli.do_action_with_exception(req)
          "
        env:
          ALIYUN_ACCESS_KEY_ID: ${{{{ secrets.ALIYUN_ACCESS_KEY_ID }}}}
          ALIYUN_ACCESS_KEY_SECRET: ${{{{ secrets.ALIYUN_ACCESS_KEY_SECRET }}}}
"#,
        title = title,
        name = name,
        kind = kind.as_str(),
        kind_resource = kind.as_str(),
        bucket = bucket,
        cdn_domain = d.cdn_domain,
        repo_name = repo_name,
        build_dir = build_dir,
        build_cmd = build_cmd,
        upload_dir = upload_dir,
    )
}

/// 各技术栈的构建命令与产物相对子目录（相对 scope 目录）。
fn build_parts(stack: DeployStack) -> (String, &'static str) {
    match stack {
        DeployStack::Vite => ("npm ci && npm run build".to_string(), "dist"),
        DeployStack::Flutter => ("flutter pub get && flutter build web".to_string(), "build/web"),
        DeployStack::Myst => ("pip install mystmd && myst build --html".to_string(), "_build/html"),
        DeployStack::Go => ("go build ./...".to_string(), ""),
        DeployStack::Python => ("python -m build".to_string(), "dist"),
        DeployStack::Rust => ("cargo build --release".to_string(), ""),
    }
}

fn render_docker_workflow(kind: DeployKind) -> String {
    format!(
        r#"# 部署 {kind}：推送 {kind}/* tag 时构建并推送 Docker 镜像到 Docker Hub。
#
# 所需 GitHub secrets（org 级）：
#   - DOCKERHUB_USERNAME / DOCKERHUB_PASSWORD
name: Deploy {kind}

on:
  push:
    tags:
      - '{kind}/**'

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Login to Docker Hub
        uses: docker/login-action@v3
        with:
          username: ${{{{ secrets.DOCKERHUB_USERNAME }}}}
          password: ${{{{ secrets.DOCKERHUB_PASSWORD }}}}
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: |
            quanttide/{kind}:\${{{{ github.ref_name }}}}
            quanttide/{kind}:latest
"#,
        kind = kind.as_str(),
    )
}

// ═══════════════════════════════════════════════════════════════════
// Terraform 渲染
// ═══════════════════════════════════════════════════════════════════

fn render_cdn_tf(kind: DeployKind, bucket: &str, d: &DomainParts) -> String {
    let k = kind.as_str();
    format!(
        r#"# =============================================================================
# {k} CDN + DNS（{cdn_domain}）
#
# 链路：OSS 静态网站桶（{k}-bucket.tf，私有）→ CDN 加速 + 私有回源鉴权
#   → CNAME 接入（云解析）→ 用户浏览器
#
# 说明：
#   - 桶 ACL 私有（RAM 用户无权限设公共读），回源鉴权分两步：
#     ① 账号级授权（本文件 RAM 角色/策略，对齐阿里云官方文档命名）
#     ② 域名级开关：CDN 控制台「回源配置 → 阿里云OSS私有Bucket回源」开启，
#        回源类型选「同账号回源（STS）」——该开关无公开 OpenAPI，
#        且与 OSS 静态网站托管默认首页存在已知冲突。
#   - 证书：复用泛域名证书 *.quanttide.com。证书 ID 需在证书服务查询后
#     填入 certificate_config；未配置前域名仅 HTTP 可用。
#   - 前置：quanttide.com 已完成 ICP 备案。
# =============================================================================

resource "alicloud_cdn_domain_new" "{k}" {{
  domain_name = "{cdn_domain}"
  cdn_type    = "web"

  sources {{
    content  = "{bucket}.oss-cn-hangzhou.aliyuncs.com"
    type     = "oss"
    port     = 80
    priority = 20
  }}

  # HTTPS 证书：由 acme.sh 管理（*.quanttide.com 泛域名证书，90 天自动续期），
  # terraform 不管理证书内容（避免私钥入库）。{cdn_domain} 为单层子域，泛域名证书可直接覆盖。
  # certificate_config {{
  #   cert_type              = "upload"
  #   server_certificate     = "<PEM 公钥，acme.sh 签发>"
  #   private_key            = "<PEM 私钥>"
  #   server_certificate_status = "on"
  # }}
}}

# 私有 Bucket 回源开关（l2_oss_key：private_oss_auth=on，自动 STS 同账号回源）
resource "alicloud_cdn_domain_config" "{k}_private_back" {{
  domain_name   = alicloud_cdn_domain_new.{k}.domain_name
  function_name = "l2_oss_key"
  function_args {{
    arg_name  = "private_oss_auth"
    arg_value = "on"
  }}
}}

# SPA 回退改写：子路由直接访问/刷新回源 OSS 404，统一改写为 /index.html
resource "alicloud_cdn_domain_config" "{k}_spa_fallback" {{
  domain_name   = alicloud_cdn_domain_new.{k}.domain_name
  function_name = "back_to_origin_url_rewrite"
  function_args {{
    arg_name  = "source_url"
    arg_value = "^/(?!index\\.html$|assets/|vite\\.svg$).*"
  }}
  function_args {{
    arg_name  = "target_url"
    arg_value = "/index.html"
  }}
  function_args {{
    arg_name  = "flag"
    arg_value = "break"
  }}
}}

# 强制 HTTPS：HTTP 请求 301 跳转 HTTPS
resource "alicloud_cdn_domain_config" "{k}_https_force" {{
  domain_name   = alicloud_cdn_domain_new.{k}.domain_name
  function_name = "https_force"
  function_args {{
    arg_name  = "enable"
    arg_value = "on"
  }}
}}

# ── 账号级授权：CDN 回源私有 OSS（阿里云官方命名，幂等） ─────────────

resource "alicloud_ram_policy" "cdn_private_oss" {{
  policy_name     = "AliyunCDNAccessingPrivateOSSRolePolicy"
  description     = "用于CDN/DCDN回源私有OSS Bucket角色的授权策略"
  policy_document = <<-EOT
    {{
      "Version": "1",
      "Statement": [
        {{ "Action": ["oss:List*", "oss:Get*"], "Resource": "*", "Effect": "Allow" }}
      ]
    }}
  EOT
}}

resource "alicloud_ram_role" "cdn_private_oss" {{
  name        = "AliyunCDNAccessingPrivateOSSRole"
  description = "用于CDN回源私有OSS Bucket"
  document    = <<-EOT
    {{
      "Statement": [
        {{
          "Action": "sts:AssumeRole",
          "Effect": "Allow",
          "Principal": {{ "Service": ["cdn.aliyuncs.com"] }}
        }}
      ],
      "Version": "1"
    }}
  EOT
}}

resource "alicloud_ram_role_policy_attachment" "cdn_private_oss" {{
  role_name   = alicloud_ram_role.cdn_private_oss.name
  policy_name = alicloud_ram_policy.cdn_private_oss.policy_name
  policy_type = "Custom"
}}

# ── DNS：CNAME 接入 ────────────────────────────────────────────────

resource "alicloud_alidns_record" "{k}" {{
  domain_name = "quanttide.com"
  rr          = "{rr}"
  type        = "CNAME"
  value       = alicloud_cdn_domain_new.{k}.cname
  ttl         = 600
}}
"#,
        k = k,
        cdn_domain = d.cdn_domain,
        bucket = bucket,
        rr = d.rr,
    )
}

fn render_bucket_tf(kind: DeployKind, bucket: &str) -> String {
    let k = kind.as_str();
    format!(
        r#"# {k} 静态站点桶（{bucket}）
#
# 部署链路（.github/workflows/deploy-{k}.yml）：
#   {k}/* tag → Actions（terraform apply + build + ossutil cp）→ 本桶（静态网站模式）
#   → 阿里云 CDN（{bucket}）

resource "alicloud_oss_bucket" "{k}" {{
  bucket            = local.oss_bucket
  storage_class     = "Standard"
  resource_group_id = data.terraform_remote_state.platform.outputs.resource_group_id
  tags = {{
    project     = var.project
    environment = var.environment
  }}

  # 静态网站托管：index.html 为入口
  website {{
    index_document = "index.html"
    error_document = "index.html"
  }}
}}
"#,
        k = k,
        bucket = bucket,
    )
}

fn render_locals_tf(kind: DeployKind, bucket: &str) -> String {
    let k = kind.as_str();
    format!(
        r#"locals {{
  # 应用级资源命名：<app>-<env>（系统级资源由 quanttide-platform 管理）
  app_name_prefix = "${{var.project}}-${{var.environment}}"

  # {k} 桶：命名对齐站点规范 {{repo}}-{k}（如 {bucket}）；OSS 全局唯一
  oss_bucket = var.oss_bucket_name
}}
"#,
        k = k,
        bucket = bucket,
    )
}

fn render_outputs_tf(kind: DeployKind) -> String {
    let k = kind.as_str();
    format!(
        r#"output "oss_bucket" {{
  description = "{k} 桶名"
  value       = alicloud_oss_bucket.{k}.bucket
}}

output "cdn_domain" {{
  description = "CDN 域名"
  value       = alicloud_cdn_domain_new.{k}.domain_name
}}

output "cdn_cname" {{
  description = "CDN CNAME"
  value       = alicloud_cdn_domain_new.{k}.cname
}}
"#,
        k = k,
    )
}

fn render_platform_tf() -> String {
    r#"# 系统级资源引用：由 quanttide-platform 仓库管理（资源组）
data "terraform_remote_state" "platform" {
  backend = "oss"
  config = {
    bucket = "quanttide-terraform-state"
    key    = "quanttide-platform/terraform.tfstate"
    region = "cn-hangzhou"
  }
}
"#
    .to_string()
}

fn render_providers_tf() -> String {
    r#"# 阿里云凭证通过环境变量注入（不在代码中写死）：
#   export ALICLOUD_ACCESS_KEY=...
#   export ALICLOUD_SECRET_KEY=...
provider "alicloud" {
  region = var.region
}

terraform {
  backend "oss" {}
}
"#
    .to_string()
}

fn render_variables_tf(bucket: &str) -> String {
    format!(
        r#"variable "region" {{
  description = "阿里云地域"
  type        = string
  default     = "cn-hangzhou"
}}

variable "project" {{
  description = "项目名（资源命名前缀）"
  type        = string
  default     = "app"
}}

variable "environment" {{
  description = "环境：dev / prod"
  type        = string
  default     = "prod"
}}

variable "oss_bucket_name" {{
  description = "站点桶名（OSS 全局唯一；静态网站模式）"
  type        = string
  default     = "{bucket}"
}}

variable "image" {{
  description = "保留变量（对齐平台 deploy-site 模板；本站点不使用）"
  type        = string
  default     = ""
}}
"#,
        bucket = bucket,
    )
}

fn render_versions_tf() -> String {
    r#"terraform {
  required_version = ">= 1.5"

  required_providers {
    alicloud = {
      source  = "aliyun/alicloud"
      version = "~> 1.240"
    }
  }
}
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在临时目录写入 contract.yaml，返回含 scopes 的仓库根。
    fn contract_repo(scope_list: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let cfg_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("contract.yaml"),
            format!(
                "scopes:\n{}\n",
                scope_list
            ),
        )
        .unwrap();
        d
    }

    fn opts(domain: &str, scope: &str, repo_path: &Path) -> InitOptions {
        InitOptions {
            domain: domain.to_string(),
            scope: scope.to_string(),
            bucket: None,
            repo: Some("qtcloud-devops".to_string()),
            force: false,
            dry_run: true,
            repo_path: repo_path.to_path_buf(),
        }
    }

    const SCOPES: &str = "  cli:\n    dir: src/cli\n    language: rust\n  provider:\n    dir: src/provider\n    language: go\n  studio:\n    dir: src/studio\n    language: dart\n";

    #[test]
    fn test_parse_domain_subdomain() {
        let d = parse_domain("crowd.quanttide.com").unwrap();
        assert_eq!(d.rr, "crowd");
        assert_eq!(d.cdn_domain, "crowd.quanttide.com");
    }

    #[test]
    fn test_parse_domain_apex() {
        let d = parse_domain("quanttide.com").unwrap();
        assert_eq!(d.rr, "@");
        assert_eq!(d.cdn_domain, "quanttide.com");
    }

    #[test]
    fn test_parse_domain_with_scheme() {
        let d = parse_domain(" https://studio.quanttide.com/ ").unwrap();
        assert_eq!(d.rr, "studio");
        assert_eq!(d.cdn_domain, "studio.quanttide.com");
    }

    #[test]
    fn test_parse_domain_invalid() {
        assert!(parse_domain("nope").is_err());
    }

    #[test]
    fn test_kind_from_scope_by_name() {
        assert_eq!(DeployKind::from_scope("studio", "dart"), Some(DeployKind::Studio));
        assert_eq!(DeployKind::from_scope("provider", "go"), Some(DeployKind::Provider));
        assert_eq!(DeployKind::from_scope("docs", "python"), Some(DeployKind::Docs));
    }

    #[test]
    fn test_kind_from_scope_by_language() {
        // 名称不匹配时按语言回退
        assert_eq!(DeployKind::from_scope("web", "dart"), Some(DeployKind::Studio));
        assert_eq!(DeployKind::from_scope("api", "go"), Some(DeployKind::Provider));
        assert_eq!(DeployKind::from_scope("site", "typescript"), Some(DeployKind::Site));
    }

    #[test]
    fn test_init_scope_studio() {
        let d = contract_repo(SCOPES);
        let r = init(&opts("studio.quanttide.com", "studio", d.path())).unwrap();
        assert_eq!(r.kind, DeployKind::Studio);
        assert_eq!(r.stack, DeployStack::Flutter);
        assert_eq!(r.bucket, "qtcloud-devops-studio");
        assert_eq!(r.scope_dir, "src/studio");
        assert_eq!(r.files.len(), 9);
        assert!(r.files.iter().any(|f| f.path == PathBuf::from(".github/workflows/deploy-studio.yml")));
        let wf = r.files.iter().find(|f| f.path.to_string_lossy().contains("deploy-studio.yml")).unwrap();
        assert!(wf.content.contains("flutter build web"));
        // 上传路径基于 scope 目录
        assert!(wf.content.contains("src/studio/build/web/"));
        assert_eq!(r.cdn_domain, "studio.quanttide.com");
    }

    #[test]
    fn test_init_scope_provider_docker() {
        let d = contract_repo(SCOPES);
        let r = init(&opts("api.quanttide.com", "provider", d.path())).unwrap();
        assert_eq!(r.kind, DeployKind::Provider);
        assert_eq!(r.stack, DeployStack::Go);
        // provider 非静态：仅 workflow
        assert_eq!(r.files.len(), 1);
        let wf = &r.files[0];
        assert!(wf.path.to_string_lossy().contains("deploy-provider.yml"));
        assert!(wf.content.contains("docker/build-push-action"));
    }

    #[test]
    fn test_init_scope_cli_derives_site() {
        // cli scope（rust）→ 名称不匹配，语言回退为 Site
        let d = contract_repo(SCOPES);
        let r = init(&opts("cli.quanttide.com", "cli", d.path())).unwrap();
        assert_eq!(r.kind, DeployKind::Site);
        assert_eq!(r.stack, DeployStack::Rust);
    }

    #[test]
    fn test_init_scope_not_found() {
        let d = contract_repo(SCOPES);
        assert!(matches!(
            init(&opts("x.quanttide.com", "missing", d.path())),
            Err(DeployError::ScopeNotFound(_))
        ));
    }
}
