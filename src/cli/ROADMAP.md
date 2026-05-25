# ROADMAP

## v0.3.1（CI 失败，未发布）

## v0.3.2 — release status ✅

新增 `release status` 命令，查看发布状态。修复 `python.rs` 模块路径。

## v0.3.x — code 命令体验修复 ✅

基于 `v0.3.0` 实际使用体验（17 个子模块全流程实测），修复 `code status` 和 `code sync` 的核心问题。

### P0 — 状态误判与实时性 ✅

- [x] 修正状态判定：工作区有未提交修改才标 Dirty，父指针落后标 AheadOfParent
- [x] `status` 默认先 fetch，获取远程最新 ref
 - [x] `--offline` 参数跳过 fetch

### P1 — CLI 设计 ✅

- [x] `--dry-run` 下放到 `sync` / `retire` 各子命令

### P2 — 输出格式 ✅

- [x] 同步输出改为单行聚合格式
- [x] 失败的子模块显式标记

### cancel 废弃 ✅

- [x] clap 注释标记废弃，执行时打印 deprecation warning

## v0.4.x — 移除 cancel & stage 关联预发布 & 发布目标支持

### 移除 cancel

- `cancel` 命令移除，有需要时使用 `retire` 替代

### 发布流程重构

当前 `stage` 只校验版本号，`publish` 同时打 tag 和创建 Release。`stage` 和预发布（rc）没有关联。

改进方向：

```
stage -v cli/v0.3.2-rc.1      ← 标记 rc，推送 tag，触发 CI
  → CI 验证 rc 构建
  → 通过后 publish -v cli/v0.3.2    ← 正式版：打 tag + GitHub Release + 注册源
  → 失败后直接 stage -v cli/v0.3.2-rc.2  ← 递增 rc 序号

- [ ] `stage` 改为推送 rc tag（版本号含 `-rc.N` 后缀），触发 CI
- [ ] `publish` 保持正式版打 tag + GitHub Release 不变
- [ ] CI 应支持 rc tag 的构建（当前基于 Release 事件触发，需改为 tag push 或增加 rc 构建 workflow）

### 发布目标支持

- **pub.dev 发布集成**：release 命令支持发布到 pub.dev
- **发布目标抽象**：从 PyPI/pub.dev 的具体实现中提取"发布目标"模型

## 待规划

### P1 — 体验修复

- **Orphaned 状态拆分**（推迟自"开发中"）：将 `Orphaned` 拆分为更精确的子状态（rebase force push、squash merge、仓库替换等），更新 `RepoState::scan()` 判定逻辑和 `describe_issue()` 建议

### P2 — 配置扩展

- 放宽分支限制（可配置允许的分支列表）
- 支持非 semver 版本策略
- CI Action 版本升级（Node.js 20 弃用）
- GitLink 镜像容灾同步

## 基本假设

| # | 假设 | 说明 |
|---|------|------|
| 1 | GitHub 为中心 | 主开发在 GitHub，使用 `gh` CLI。GitLink 仅作镜像容灾 |
| 2 | 有 CHANGELOG.md | 格式 `## [X.Y.Z]`，默认查找当前目录 |
| 3 | semver 版本号 | 版本号 `vX.Y.Z` 或 `scope/vX.Y.Z` |
| 4 | 工作区干净 | 发布前无未提交变更 |
| 5 | 发布分支受限 | 仅 `main` / `master` / `release/*` 可发布 |
| 6 | git remote 可达 | 从 `git remote get-url origin` 自动检测仓库 |
| 7 | 用户可交互 | 发布确认需 TTY 交互，CI 需 `-y` 跳过 |
