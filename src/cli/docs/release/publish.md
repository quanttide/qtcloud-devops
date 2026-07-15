# release 命令设计

## 定位

发布管理：版本检测、CHANGELOG 管理、git tag 创建、GitHub Release 创建。对应 DevOps 生命周期中的 **Release** 阶段。

## 命令

```
qtcloud-devops release publish                       自动检测版本 + 发布
qtcloud-devops release publish -v cli/v0.10.0-rc.1   指定版本
qtcloud-devops release publish --dry-run              仅预览不执行
qtcloud-devops release publish -y                     跳过确认
qtcloud-devops release publish -f                     强制重新发布
qtcloud-devops release publish --registry crates      指定 CI 目标
qtcloud-devops release status                         查看发布状态
qtcloud-devops release audit                          发布预检审计
qtcloud-devops release audit -v cli/v0.10.0           指定版本审计
qtcloud-devops release audit --scope cli              按 scope 过滤审计
```

## 模块架构

```
release/
├── mod.rs         re-exports + 工具函数（tag/gh/release 操作）
├── publish.rs     发布主流程
├── audit.rs       发布预检审计（版本/配置/CHANGELOG/工作区/tag/远程/GH Release）
├── detect.rs      版本自动检测（LLM + fallback heuristic）
├── status.rs      发布状态查看
└── (source/changelog.rs — CHANGELOG 生成与校验，位于外层 source 模块)
```

## release publish — 发布版本

### 执行流程

```
1. 确定版本号
   ├── 有 -v → 校验格式
   └── 无 -v → detect_version() 自动检测
       ├── 读取 scope 的最新 tag
       ├── 统计未发布提交数量
       ├── LLM 决策（quanttide-agent）或 fallback heuristic
       └── 建议版本号

2. 解析 scope（从版本号提取 scope 名，查契约得到子目录）

3. 更新配置文件版本号
   ├── Cargo.toml
   └── pyproject.toml

4. 一致性检查
   ├── git add 配置文件
   ├── cargo generate-lockfile 同步 Cargo.lock
   └── 验证所有配置文件版本一致

5. CHANGELOG 管理
   ├── 自动生成缺失条目（LLM）
   └── 验证 CHANGELOG 包含版本记录

6. 用户确认（除非 -y）

7. 执行
   ├── git tag {version}
   ├── git push origin {version}
   ├── gh release create {version}
   └── 打印 GitHub Release URL
```

### 版本号格式

- 有 scope: `cli/v0.1.0`、`cli/v0.1.0-rc.1`
- 无 scope: `v0.1.0`、`v0.1.0-alpha.1`

### 自动版本检测逻辑

1. 从契约读取 scope（或自动检测）
2. 获取该 scope 的最新 tag
3. 统计自最新 tag 以来的 commit 数
4. 用 `quanttide-agent` LLM 决策版本增量（major/minor/patch/prerelease）
5. LLM 不可用时用 fallback heuristic（feat → minor, fix → patch, breaking → major）
6. 预发布版本按 stage 递增（alpha → beta → rc → formal）

### force 模式（-f）

清理已存在的 tag 和 GitHub Release 后重新发布：

1. `gh release delete {version}`
2. `git push --delete origin {version}`
3. `git tag -d {version}`
4. 从头执行发布流程

### dry-run 模式

仅预览，不执行任何 git/config 操作：

```
💡 预览发布: cli/v0.10.0-rc.1
   将更新 Cargo.toml/pyproject.toml 版本号
   将更新 CHANGELOG.md
   将创建 git tag 并推送到远端
   将创建 GitHub Release
   使用 -y 跳过确认直接发布
```

## release status — 查看发布状态

逐 scope 展示发布状态。调用 `collect_all()` 组合 scope 配置、git 标签、CHANGELOG 检查，产出各 scope 的 `ReleaseState` 快照。

```
发布状态
────────────────────────────────────────
  [qtcloud-core]
    状态:         待发布
    路径:         apps/qtcloud-core
    最新标签:     v2.1.0
    未发布提交:   3
    变更日志:     CHANGELOG.md
    版本一致:     是

  [(root)]
    状态:         已是最新
    路径:         .
    最新标签:     v5.0.0
    未发布提交:   0
    变更日志:     CHANGELOG.md
    版本一致:     是

  [studio]
    状态:         未发布
    路径:         apps/studio
    最新标签:     (无)
    未发布提交:   0
    变更日志:     CHANGELOG.md
```

### `ReleaseStatus` 枚举

| 变体 | 标签 | 含义 |
|------|------|------|
| `Unreleased` | 未发布 | scope 未匹配到 tag |
| `Latest` | 已是最新 | tag 存在，无未发布提交，CHANGELOG 一致 |
| `Pending` | 待发布 | 自 tag 以来有新提交 |
| `Inconsistent` | 版本冲突 | CHANGELOG 中缺失对应版本条目 |
| `Unknown` | 状态未知 | git 命令失败 |

### 状态判定逻辑

```
有 tag → CHANGELOG 版本缺失 → Inconsistent
       └── 有新提交 → Pending
           └── 无新提交 → Latest
无 tag → Unreleased
```

## 公共 API

```rust
// publish
pub fn publish(
    version: Option<&str>,     // None = auto-detect
    repo_path: &Path,
    yes: bool,                  // 跳过确认
    force: bool,               // 强制重新发布
    dry_run: bool,
    registry: Option<PublishTarget>,
) -> Result<(), Box<dyn std::error::Error>>

// publish
pub fn publish(
    version: Option<&str>,
    repo_path: &Path,
    yes: bool,
    force: bool,
    dry_run: bool,
    registry: Option<PublishTarget>,
) -> Result<(), Box<dyn std::error::Error>>

// audit
pub fn audit(version: Option<&str>, repo_path: &Path) -> Result<Vec<AuditItem>, String>
pub fn audit_all(repo_path: &Path, scope_filter: Option<&str>)
    -> Result<Vec<(String, Vec<AuditItem>)>, String>
pub struct AuditItem {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

// detect
pub fn detect_version(repo_path: &Path) -> Result<DetectResult, DetectError>
pub struct DetectResult { pub version: String }
pub enum DetectError {
    Git(String), Llm(String), Version(String), Other(String),
}

// status
pub fn status(repo_path: &Path)
pub fn collect_all(repo_path: &Path) -> Vec<ReleaseState>
pub use quanttide_devops::stage::release::{ReleaseState, ReleaseStatus}

// changelog (位于 crate::source::changelog)
pub fn ensure_changelog(repo_path: &Path, scope_dir: &Path, version: &str)
    -> Result<Option<String>, ChangelogError>

// mod.rs 公共函数
pub fn validate_version(version: &str) -> bool
pub fn normalize_version(version: &str) -> String
pub fn precheck_version_changelog(version: &str, changelog_path: &Path) -> Vec<String>
pub fn extract_notes(version: &str, changelog_path: &Path) -> Option<String>
pub fn confirm_release(version: &str, yes: bool) -> bool
pub fn create_tag(version: &str, repo_path: &Path) -> bool
pub fn push_tag(version: &str, repo_path: &Path) -> Result<(), String>
pub fn create_release(version: &str, notes: &str, repo: &str) -> bool
pub fn delete_local_tag(version: &str, repo_path: &Path) -> bool
pub fn delete_remote_tag(version: &str, repo_path: &Path) -> bool
pub fn delete_release(version: &str, repo: &str) -> bool
pub fn rollback_tag(version: &str, repo_path: &Path)
pub fn get_remote_repo(repo_path: &Path) -> Option<String>
pub fn parse_github_repo(url: &str) -> Option<String>
pub fn load_scopes_map(repo_path: &Path) -> HashMap<String, String>
pub fn get_latest_tags_by_scope(repo_path: &Path) -> Vec<(String, String)>
pub fn resolve_scope_dir(version: &str, repo_path: &Path) -> PathBuf

pub enum PublishTarget { Crates, PyPI, PubDev }
```
