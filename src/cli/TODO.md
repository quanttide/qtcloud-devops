# TODO

v0.3.x 全部完结。无遗留项。

## v0.3.x — code 命令修复 ✅

### P0 — 状态误判 ✅

- [x] 修正 `RepoState::scan()`：工作区脏才标 Dirty，父指针落后标 AheadOfParent
  - 测试：`test_scan_with_ahead_via_remote_unreachable` 验证 AheadOfParent

### P0 — remote_head fetch ✅

- [x] `status` 执行时默认先 fetch（git2::Remote::fetch）
- [x] 增加 `--offline` 参数跳过 fetch

### P1 — CLI 设计 ✅

- [x] `--dry-run` 下放到 `sync` / `retire` 各子命令级别

### P2 — 输出格式 ✅

- [x] 同步输出改为单行聚合格式：`name  ✓ push · sync · ✓ push-parent`
- [x] 失败的子模块显式标记：`✗`

### P3 — cancel 废弃 ✅

- [x] clap 注释标记已废弃
- [x] 执行时打印 deprecation warning
