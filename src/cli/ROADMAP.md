# ROADMAP

## v0.8.0（当前）：

- [x] `contract.rs` 重构 — 类型定义委托给 `quanttide-devops` toolkit
  - [x] 移除 YAML 镜像类型（~600 行）
  - [x] `Registry` 拆分：契约配置用 `contract::Registry`，CLI 参数用 `PublishTarget`
  - [x] 修复 `extract_json_version` 单行 JSON 解析 bug
  - [x] 删除全部 toolkit 已提供的重复代码（~160 行 + 20 测试）

## v0.7.0（已完成）：

- [x] 依赖 `quanttide-devops = "0.1"`（crates.io 发布）
- [x] 项目管理
- [x] `release` 模块
- [x] `code` 模块

---

## 待定

- `code` ↔ `submodule` 拆分
  - `code sync` — 纯业务抽象，封装 submodule 细节
  - `code status` — 输出业务状态（Synced / PendingPush / PendingPull / Conflict）
  - `submodule status` — 调试入口，暴露七态
  - `submodule sync` — 底层操作（debug 用）
  - `model/code.rs` 剥离 submodule 概念，换为 `SyncStatus`
