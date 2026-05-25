# TODO

## v0.3.x — code 命令修复

基于 17 个子模块全流程实测发现的问题。

### P0 — 状态误判

**问题**：`code status` 将纯 AheadOfParent 的子模块标记为 Dirty。根因是状态判定混用了"工作区是否干净"和"父指针是否落后"两个维度。Dirty 应仅指工作区有未提交修改。

**修复**：

- [ ] 修正 `RepoState::scan()` 判定逻辑：工作区脏（`is_dirty`）才标 Dirty，父指针落后（`ahead_count > 0`）标 AheadOfParent
  - 文件：`src/model/code.rs`，`RepoState::scan()` 中的状态选择分支（当前约 line 189-205）
  - 当前逻辑：`is_dirty` 优先于 `ahead_count`，导致带新提交的子模块被标为 Dirty
  - 改为：`is_dirty` 仅在工作区有未提交修改时为 true（`raw_status.is_wd_modified()`），不因 ahead_count > 0 而触发
  - 测试：验证子模块仅有 ahead commit 时状态为 AheadOfParent，不是 Dirty

### P0 — remote_head 是本地缓存

**问题**：三路比对中的 `remote_head` 是本地缓存 `refs/remotes/origin/main`，不反映远程实时状态。`BehindRemote` 永远检测不到真正的远程更新，因为没 fetch。

**修复**：

- [ ] `status` 执行时默认先 `git fetch`，获取远程最新 ref
  - 文件：`src/commands/code.rs`，`status()` 逻辑入口处
  - 使用 `git fetch --all --quiet --no-tags`（静默、不拉 tag、速度快）
  - fetch 失败时降级到本地缓存，输出的对应行标记 🛰（如 `BehindRemote 🛰`）
- [ ] 增加 `--offline` 参数跳过 fetch
  - `code status --offline` 使用纯本地缓存，不发起网络请求
  - 适用场景：离线开发、CI 中不需要实时状态
- [ ] 测试：有网络的场景验证 fetch 后 remote_head 正确更新；验证 `--offline` 不发起 fetch

### P1 — CLI 设计

**问题**：`--dry-run` 是 `code` 级别的选项，而非 `sync` / `status` 子命令级别：

```
# 当前（违反直觉）
qtcloud-devops code --dry-run sync

# 期望
qtcloud-devops code sync --dry-run
```

**修复**：

- [ ] 将 `--dry-run` 从 `Commands::Code` 移到 `CodeAction::Sync`、`CodeAction::Status`、`CodeAction::Retire`
  - 文件：`src/main.rs` `CodeAction` 枚举定义
  - 当前：`Code { dry_run: bool, action: CodeAction }`
  - 改为：各子命令自己声明 `#[arg(long)] dry_run: bool`
  - 影响的匹配分支统一调整
- [ ] 测试：各子命令的 `--dry-run` 参数通过编译且行为正确

### P2 — 输出格式

**问题**：同步 17 个子模块时生成 51 行日志（每行三段式），信息密度低，滚动困难。

**修复**：

- [ ] 同步输出改为单行聚合格式：`name  ✓ push · sync · push-parent`
  - 文件：`src/commands/code.rs`，`sync_to_parent()` 和 `sync_all_to_parent()` 输出逻辑
  - 每个子模块一行，三个阶段用 ` · ` 分隔，成功标 `✓`
- [ ] 失败的子模块显式标记：`✗ push: 权限不足 · 已跳过`
  - 失败阶段标 `✗` 并附简要错误原因
- [ ] 测试：17 个子模块输出不超过 20 行；失败项目显式标记

---

## v0.4.x — stage 关联预发布

### 发布流程重构

**问题**：当前 `stage` 只校验版本号，`publish` 同时打 tag + 创建 Release。`stage` 和预发布（rc）没有关联，`publish` 在 CI 运行前就已创建 tag 和 Release，CI 失败无法撤回。

**目标流程**：

```
stage -v cli/v0.3.2-rc.1      ← 标记 rc，推送 rc tag，触发 CI
  → CI 验证 rc 构建+测试
  → 通过后 publish -v cli/v0.3.2    ← 创建正式 tag + Release + 注册源
  → 失败后 stage -v cli/v0.3.2-rc.2  ← 递增 rc，不清理旧 tag
```

**任务**：

- [ ] `stage` 改为推送 rc tag，触发 CI（当前只校验+存 journal）
  - 文件：`src/commands/release.rs`，`stage()` 函数
  - 新增：`create_tag()` + `push_tag()`（当前仅 `publish` 调用了这些）
  - 版本号含 `-rc.N` 后缀时视为预发布，推送 tag
  - 版本号不含 `-rc.N` 时拒绝（不允许直接 stage 正式版）
- [ ] `publish` 不做 tag，只创建 GitHub Release + 发布注册源
  - 文件：`src/commands/release.rs`，`publish()` 函数
  - 移除 `create_tag()` 和 `push_tag()` 调用（tag 由 stage 推送）
  - 前置条件：版本对应的 rc tag 已存在（验证 git tag）
- [ ] `cancel` 退化为纯审计标记，不操作 git/gh
  - 文件：`src/commands/release.rs`，`cancel()` 函数
  - 移除 `rollback_tag()` 和 `gh release delete` 调用
  - 只在 journal 中记录状态为 Cancelled
- [ ] CI 新增 rc tag 构建 workflow
  - 文件：`.github/workflows/build-rc.yml`
  - 触发：`push: tags: ['cli/*-rc.*']`
  - 行为：构建 + 测试（不发布）
  - 注意：当前 CI 基于 `release: [published]` 触发，需增加 tag push 触发

---

## v0.4.x — 发布目标

- [ ] **pub.dev 发布集成**：release 命令支持发布到 pub.dev
  - 类似 PyPI 发布流程，调用 `dart pub publish`
- [ ] **发布目标抽象**：从 PyPI/pub.dev 的具体实现中提取"发布目标" trait
  - `PublishTarget` trait：`publish(version, artifact_path) -> Result<()>`
  - 实现：`PypiTarget`、`PubDevTarget`、`CratesTarget`
