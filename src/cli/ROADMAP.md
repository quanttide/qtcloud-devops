# ROADMAP

## v0.3.x — code 命令体验修复

基于 `v0.3.0` 实际使用体验（17 个子模块全流程实测），修复 `code status` 和 `code sync` 的核心问题。

### P0 — 状态误判与实时性

**状态误判**：`code status` 将纯 AheadOfParent 的子模块标记为 Dirty。根因是状态判定混用了"工作区是否干净"和"父指针是否落后"两个维度。

- [ ] 修正状态判定：工作区有未提交修改才标 Dirty，父指针落后标 AheadOfParent
- 涉及：`src/model/code.rs` `RepoState::scan()` 判定逻辑

**remote_head 是本地缓存**：三路比对中的 `remote_head` 是上次 `git fetch` 时的快照，不反映远程实时状态。`BehindRemote` 无法检测真正的远程更新。

- [ ] `status` 默认先 fetch，失败时降级到本地缓存并标记 🛰
- [ ] 增加 `--offline` 参数跳过 fetch
- 涉及：`src/commands/code.rs` `status` 逻辑

### P1 — CLI 设计

- [ ] `--dry-run` 下放到 `sync` / `status` / `retire` 各子命令（当前是 `code` 级别，违反直觉）
- 涉及：`src/main.rs` `CodeAction` 参数定义

### P2 — 输出格式

- [ ] 同步输出改为单行聚合格式：`name  ✓ push · sync · push-parent`
- [ ] 失败的子模块显式标记：`✗ push: 权限不足 · 已跳过`
- 涉及：`src/commands/code.rs` 输出逻辑

## v0.4.0 — 发布目标支持

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
