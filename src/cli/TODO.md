# TODO

## v0.3.x — code 命令修复

### P0 — 状态误判

- [ ] 修正 `RepoState::scan()`：工作区脏才标 Dirty，父指针落后标 AheadOfParent
- [ ] `status` 默认先 fetch，失败时降级到本地缓存并标记 🛰
- [ ] 增加 `--offline` 参数跳过 fetch

### P1 — CLI 设计

- [ ] `--dry-run` 下放到 `sync` / `status` / `retire` 各子命令

### P2 — 输出格式

- [ ] 同步输出改为单行聚合格式：`name  ✓ push · sync · push-parent`
- [ ] 失败的子模块显式标记：`✗ push: 权限不足 · 已跳过`

---

## v0.4.x — stage 关联预发布

- [ ] `stage` 改为推送 rc tag（版本号含 `-rc.N`），触发 CI
- [ ] `publish` 不做 tag，只创建 GitHub Release（tag 由 stage 推送）
- [ ] `cancel` 退化为审计标记，不操作 git/gh
- [ ] CI 新增 rc tag 构建 workflow（当前基于 Release 事件触发）

## v0.4.x — 发布目标

- [ ] pub.dev 发布集成
- [ ] 发布目标抽象模型
