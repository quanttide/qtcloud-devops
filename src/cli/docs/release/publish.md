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
```

## 模块架构

```
release/
├── mod.rs         re-exports
├── publish.rs     发布主流程
├── detect.rs      版本自动检测（LLM + fallback heuristic）
├── status.rs      发布状态查看
├── changelog.rs   CHANGELOG 生成与校验
└── util.rs        工具函数（tag/gh/release 操作）
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

展示当前仓库的发布状态：

```
发布状态
────────────────────────────────────────────────
  tag:          cli/v0.9.3-alpha.3
  version:      0.9.3-alpha.3
  CHANGELOG:    ✅ 包含 0.9.3-alpha.3 条目
  GitHub Release: ✅ 已发布
  工作区:       ✅ 干净
```

### 检查项

| 检查 | 来源 | 说明 |
|------|------|------|
| 最新 tag | `git tag --sort=-version:refname` | 按 scope 过滤 |
| 版本一致性 | tag vs Cargo.toml | 自动去 v 前缀比较 |
| CHANGELOG | 检查版本条目 | ✅/❌/⚠ 未找到 |
| GitHub Release | `gh release view` | ✅/❌/⚠ gh 未安装 |
| 工作区状态 | `git status --porcelain` | ✅ 干净 / ❌ 有未提交变更 |
| CI 状态 | 依赖 build status | 仅展示 scope 名 |

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

// detect
pub fn detect_version(repo_path: &Path) -> Result<DetectResult, String>
pub struct DetectResult { pub version: String }

// status
pub fn status(repo_path: &Path)

// changelog
pub fn ensure_changelog(repo_path: &Path, scope_dir: &Path, version: &str) -> Result<(), String>

// util
pub fn validate_version(version: &str) -> bool
pub fn create_tag(version: &str, repo_path: &Path) -> bool
pub fn push_tag(version: &str, repo_path: &Path) -> bool
pub fn create_release(version: &str, notes: &str, repo: &str) -> bool
pub fn delete_local_tag(version: &str, repo_path: &Path)
pub fn delete_remote_tag(version: &str, repo_path: &Path)
pub fn delete_release(version: &str, repo: &str)
pub fn rollback_tag(version: &str, repo_path: &Path)
pub fn extract_notes(version: &str, changelog_path: &Path) -> Option<String>
pub fn get_remote_repo(repo_path: &Path) -> Option<String>
pub fn parse_github_repo(url: &str) -> Option<String>
pub fn precheck_version_changelog(version: &str, changelog_path: &Path) -> Vec<String>

pub enum PublishTarget { Crates, PyPi, PubDev, GitHub }
```
