# ROADMAP

## v0.5.0:

- [x] 取消 `release retire` 命令
- [x] 取消 `release` 模块的状态（领域模型移入 toolkit）
- [x] 取消 `code retire` 命令
- [x] `code sync` 修复（签名/原子性/fetch）

---

- `code` ↔ `submodule` 拆分
  - `code sync` — 纯业务抽象，封装 submodule 细节
  - `code status` — 输出业务状态（Synced / PendingPush / PendingPull / Conflict）
  - `submodule status` — 调试入口，暴露七态
  - `submodule sync` — 底层操作（debug 用）
  - `model/code.rs` 剥离 submodule 概念，换为 `SyncStatus`
