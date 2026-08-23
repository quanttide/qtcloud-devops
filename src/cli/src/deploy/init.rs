//! deploy init — 一键为指定 scope 生成或升级部署能力。
//!
//! 输出收敛到**仓库根**，遵循「统一治理 + 最大化 IaC」：
//! - `manifests/terraform/` 只留薄声明：`providers.tf` + `variables.tf` + `platform.tf` + `main.tf`，
//!   其中 `main.tf` 是调用 `quanttide-platform` 共享模块的 `module` 块，不复制基础设施代码。
//! - `.github/workflows/deploy-{scope}.yml` 为根级 workflow，`-target=module.{scope}` 精确应用该 scope 的 IaC。
//!
//! 部署形态 / 技术栈 / scope 目录由契约 scope 推导（`DeployKind::from_scope`）。

use super::{detect_repo, DeployError, DeployKind, DeployStack};
use std::path::{Path, PathBuf};

/// 共享模块源（quanttide-platform），`////` 分隔 git URL 与子目录。
const MODULE_BASE: &str =
    "git::https://github.com/quanttide/quanttide-platform.git//manifests/terraform/modules";

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
    /// 仓库根路径（用于加载契约、推导目录与输出 IaC/workflow）。
    pub repo_path: PathBuf,
}

/// 单个生成的部署文件。
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// 相对仓库根的路径。
    pub path: PathBuf,
    /// 文件内容。
    pub content: String,
    /// 该文件在磁盘上已存在。
    pub existed: bool,
    /// 是否始终覆写（用于需合并的文件如 main.tf，不因 force 跳过）。
    pub always_write: bool,
}

/// `deploy init` 的结果报告。
#[derive(Debug)]
pub struct InitReport {
    /// 生成/预览的文件清单。
    pub files: Vec<GeneratedFile>,
    /// 推导出的 OSS 桶名（provider 下为服务名）。
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

    let suffix = format!("-{}", kind.as_str());
    let repo_name = bucket.strip_suffix(&suffix).unwrap_or(&bucket).to_string();

    let files = render_files(opts, kind, stack, &bucket, &repo_name, &scope_dir, &domain)?;

    if !opts.dry_run {
        for f in &files {
            write_if_needed(&opts.repo_path, f, opts.force)?;
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

/// 解析域名，返回 `cdn_domain`。
///
/// - `crowd.quanttide.com` → `crowd.quanttide.com`
/// - `quanttide.com`（apex）→ `quanttide.com`
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
    Ok(DomainParts { cdn_domain: domain.to_string() })
}

/// 生成全部部署文件（仓库根相对路径 + 内容）。
fn render_files(
    opts: &InitOptions,
    kind: DeployKind,
    stack: DeployStack,
    bucket: &str,
    repo_name: &str,
    scope_dir: &str,
    d: &DomainParts,
) -> Result<Vec<GeneratedFile>, DeployError> {
    let mut files = Vec::new();
    let tf_dir = PathBuf::from("manifests/terraform");

    // provider / variables / platform（仅在不存在时生成，已是薄声明）
    let tern = [
        ("providers.tf", render_providers_tf()),
        ("variables.tf", render_variables_tf(repo_name)),
        ("platform.tf", render_platform_tf()),
    ];
    for (name, content) in tern {
        let path = tf_dir.join(name);
        let existed = opts.repo_path.join(&path).exists();
        files.push(GeneratedFile { path, content, existed, always_write: false });
    }

    // main.tf：合并 module 块（保留其它 scope，`{scope}` 幂等替换）
    let main_path = tf_dir.join("main.tf");
    let main_existed = opts.repo_path.join(&main_path).exists();
    let existing = std::fs::read_to_string(opts.repo_path.join(&main_path)).unwrap_or_default();
    let module_block = render_module_block(opts.scope.as_str(), kind, bucket, repo_name, d);
    let main_content = upsert_module_block(&existing, &opts.scope, &module_block);
    files.push(GeneratedFile {
        path: main_path,
        content: main_content,
        existed: main_existed,
        always_write: true, // main.tf 需合并，始终覆写
    });

    // 根级 workflow
    let wf = PathBuf::from(format!(".github/workflows/deploy-{}.yml", opts.scope));
    let wf_existed = opts.repo_path.join(&wf).exists();
    let wf_content = render_workflow(opts.scope.as_str(), kind, stack, bucket, repo_name, scope_dir, d);
    files.push(GeneratedFile {
        path: wf,
        content: wf_content,
        existed: wf_existed,
        always_write: false,
    });

    Ok(files)
}

fn write_if_needed(base: &Path, f: &GeneratedFile, force: bool) -> Result<(), DeployError> {
    let full = base.join(&f.path);
    if full.exists() && !force && !f.always_write {
        // 已存在且未强制：跳过（main.tf 除外，其需合并）
        return Ok(());
    }
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full, &f.content)?;
    Ok(())
}

/// 将某个 module 块 upsert 进 main.tf：不存在则追加，存在则替换（保护其它 scope 的块）。
fn upsert_module_block(existing: &str, scope: &str, block: &str) -> String {
    let marker = format!("module \"{}\" {{", scope);
    let Some(start) = existing.find(&marker) else {
        // 不存在：追加
        return if existing.trim().is_empty() {
            format!("{}\n", block)
        } else {
            format!("{}\n{}\n", existing.trim_end(), block)
        };
    };
    // 找到匹配的右花括号
    let brace_start = start + marker.len();
    let bytes = existing.as_bytes();
    let mut depth = 1usize;
    let mut end = brace_start;
    let mut i = brace_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let head = existing[..start].trim_end();
    let tail = existing[end + 1..].trim_start();
    format!("{}\n{}\n{}", head, block.trim_end(), tail.trim_start())
}

// ═══════════════════════════════════════════════════════════════════
// IaC（薄声明）渲染
// ═══════════════════════════════════════════════════════════════════

fn render_providers_tf() -> String {
    r#"# 阿里云凭证：本地 ~/.aliyun/config.json；CI 由 ALIYUN_ACCESS_KEY_ID/SECRET 注入
provider "alicloud" {
  region = var.region
}

terraform {
  required_version = ">= 1.6.0"

  required_providers {
    alicloud = {
      source  = "hashicorp/alicloud"
      version = "~> 1.230.0"
    }
  }

  backend "oss" {}
}
"#
    .to_string()
}

fn render_variables_tf(project: &str) -> String {
    format!(
        r#"variable "region" {{
  description = "阿里云地域"
  type        = string
  default     = "cn-hangzhou"
}}

variable "project" {{
  description = "项目名（资源命名前缀）"
  type        = string
  default     = "{project}"
}}

variable "environment" {{
  description = "环境：dev / staging / prod"
  type        = string
  default     = "prod"
}}
"#,
        project = project,
    )
}

/// 引用 quanttide-platform 根 IaC 的远程状态（account 级输出：resource_group_id / VPC / RDS）。
fn render_platform_tf() -> String {
    r#"# 系统级资源引用：由 quanttide-platform 仓库管理
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

/// 生成对 quanttide-platform 共享模块的 `module` 块。
fn render_module_block(scope: &str, kind: DeployKind, bucket: &str, repo_name: &str, d: &DomainParts) -> String {
    if kind.is_static_site() {
        format!(
            r#"module "{scope}" {{
  source  = "{base}/static-site"
  name    = "{scope}"
  domain  = "{domain}"
  bucket  = "{bucket}"
  project = "{project}"
}}"#,
            scope = scope,
            base = MODULE_BASE,
            domain = d.cdn_domain,
            bucket = bucket,
            project = repo_name,
        )
    } else {
        format!(
            r#"module "{scope}" {{
  source            = "{base}/fc"
  name              = "{bucket}"
  image             = "{repo}/{scope}:latest"
  project           = "{repo}"
  resource_group_id = data.terraform_remote_state.platform.outputs.resource_group_id
  vpc_id            = data.terraform_remote_state.platform.outputs.vpc_id
  vswitch_ids       = [data.terraform_remote_state.platform.outputs.vswitch_id]
  security_group_id = data.terraform_remote_state.platform.outputs.security_group_id
}}"#,
            scope = scope,
            base = MODULE_BASE,
            bucket = bucket,
            repo = repo_name,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Workflow 渲染
// ═══════════════════════════════════════════════════════════════════

fn render_workflow(
    scope: &str,
    kind: DeployKind,
    stack: DeployStack,
    bucket: &str,
    repo_name: &str,
    scope_dir: &str,
    d: &DomainParts,
) -> String {
    let (name, build_dir, upload_dir, build_cmd) = if kind.is_static_site() {
        let name = match kind {
            DeployKind::Site => "Deploy Site",
            DeployKind::Studio => "Deploy Studio",
            _ => "Deploy Docs",
        };
        let (build_cmd, out_rel) = build_parts(stack);
        let build_dir = if scope_dir.is_empty() { ".".to_string() } else { scope_dir.to_string() };
        let upload_dir = if out_rel.is_empty() {
            build_dir.clone()
        } else if scope_dir.is_empty() {
            out_rel.to_string()
        } else {
            format!("{}/{}", scope_dir, out_rel)
        };
        (name.to_string(), build_dir, upload_dir, build_cmd)
    } else {
        let name = match kind {
            DeployKind::Provider => "Deploy Provider",
            _ => "Deploy Service",
        };
        (name.to_string(), ".".to_string(), String::new(), "docker build -t {image} .".to_string())
    };

    let title = format!("# 部署 {}：推送 {}/* tag 时，应用 IaC（terraform apply -target=module.{scope}）并完成发布/上传。\n# 共享模块源：{base}/static-site|fc\n# 所需 GitHub secrets（org 级）：ALIYUN_ACCESS_KEY_ID / ALIYUN_ACCESS_KEY_SECRET\n", scope, scope, scope=scope, base=MODULE_BASE);

    if kind.is_static_site() {
        format!(
            r#"{title}
name: {name}

on:
  push:
    tags:
      - '{scope}/**'

concurrency:
  group: deploy-{scope}
  cancel-in-progress: false

env:
  OSS_BUCKET: {bucket}
  OSS_ENDPOINT: oss-cn-hangzhou.aliyuncs.com
  CDN_DOMAIN: {cdn_domain}

jobs:
  deploy:
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
            -backend-config="key={repo}/{scope}.tfstate" \
            -backend-config="region=cn-hangzhou"
          terraform apply -input=false -auto-approve -target=module.{scope}

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
            scope = scope,
            bucket = bucket,
            cdn_domain = d.cdn_domain,
            repo = repo_name,
            build_dir = build_dir,
            build_cmd = build_cmd,
            upload_dir = upload_dir,
        )
    } else {
        format!(
            r#"{title}
name: {name}

on:
  push:
    tags:
      - '{scope}/**'

concurrency:
  group: deploy-{scope}
  cancel-in-progress: false

jobs:
  deploy:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: {scope_dir}
    steps:
      - uses: actions/checkout@v4
      - name: Setup terraform
        uses: hashicorp/setup-terraform@v3
      - name: Apply infrastructure (Terraform)
        working-directory: manifests/terraform
        run: |
          terraform init \
            -backend-config="bucket=${{{{ secrets.TF_STATE_BUCKET || 'quanttide-terraform-state' }}}}" \
            -backend-config="key={repo}/{scope}.tfstate" \
            -backend-config="region=cn-hangzhou"
          terraform apply -input=false -auto-approve -target=module.{scope} -var image=${{{{ github.sha }} }}
      - name: Build and push image
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ${{{{ secrets.REGISTRY }}}}/{repo}-{scope}:${{{{ github.sha }} }}
"#,
            title = title,
            name = name,
            scope = scope,
            repo = repo_name,
            scope_dir = scope_dir,
        )
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn contract_repo(scope_list: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let cfg_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(cfg_dir.join("contract.yaml"), format!("scopes:\n{}\n", scope_list)).unwrap();
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
        assert_eq!(d.cdn_domain, "crowd.quanttide.com");
    }

    #[test]
    fn test_parse_domain_apex() {
        let d = parse_domain("quanttide.com").unwrap();
        assert_eq!(d.cdn_domain, "quanttide.com");
    }

    #[test]
    fn test_parse_domain_with_scheme() {
        let d = parse_domain(" https://studio.quanttide.com/ ").unwrap();
        assert_eq!(d.cdn_domain, "studio.quanttide.com");
    }

    #[test]
    fn test_parse_domain_invalid() {
        assert!(parse_domain("nope").is_err());
    }

    #[test]
    fn test_upsert_first_block() {
        let out = upsert_module_block("", "studio", "module \"studio\" {\n  a = 1\n}");
        assert_eq!(out, "module \"studio\" {\n  a = 1\n}\n");
    }

    #[test]
    fn test_upsert_preserve_other_scopes() {
        let existing = "# header\nmodule \"provider\" {\n  b = 2\n}\n";
        let out = upsert_module_block(existing, "studio", "module \"studio\" {\n  a = 1\n}");
        assert!(out.contains("module \"provider\""));
        assert!(out.contains("module \"studio\""));
    }

    #[test]
    fn test_upsert_replace_same_scope() {
        let existing = "module \"studio\" {\n  domain = \"old\"\n}\nmodule \"provider\" {\n  b = 2\n}\n";
        let out = upsert_module_block(existing, "studio", "module \"studio\" {\n  domain = \"new\"\n}");
        assert!(out.contains("domain = \"new\""));
        assert!(!out.contains("domain = \"old\""));
        assert!(out.contains("module \"provider\""));
    }

    #[test]
    fn test_init_scope_studio_static_site_module() {
        let d = contract_repo(SCOPES);
        let r = init(&opts("studio.quanttide.com", "studio", d.path())).unwrap();
        assert_eq!(r.kind, DeployKind::Studio);
        assert_eq!(r.stack, DeployStack::Flutter);
        assert_eq!(r.bucket, "qtcloud-devops-studio");
        // providers/variables/platform/main + workflow = 5 文件
        assert_eq!(r.files.len(), 5);
        let main = r.files.iter().find(|f| f.path.to_string_lossy().ends_with("main.tf")).unwrap();
        assert!(main.content.contains("module \"studio\""));
        assert!(main.content.contains("modules/static-site"));
        assert!(main.content.contains("studio.quanttide.com"));
        let wf = r.files.iter().find(|f| f.path.to_string_lossy().contains("deploy-studio.yml")).unwrap();
        assert!(wf.content.contains("-target=module.studio"));
        assert!(wf.content.contains("src/studio/build/web/"));
    }

    #[test]
    fn test_init_scope_provider_fc_module() {
        let d = contract_repo(SCOPES);
        let r = init(&opts("api.quanttide.com", "provider", d.path())).unwrap();
        assert_eq!(r.kind, DeployKind::Provider);
        assert_eq!(r.stack, DeployStack::Go);
        let main = r.files.iter().find(|f| f.path.to_string_lossy().ends_with("main.tf")).unwrap();
        assert!(main.content.contains("module \"provider\""));
        assert!(main.content.contains("modules/fc"));
        assert!(main.content.contains("resource_group_id"));
        // 仍含 workflow + 3 个薄 IaC 文件 + main = 5
        assert_eq!(r.files.len(), 5);
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
