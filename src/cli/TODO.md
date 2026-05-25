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
