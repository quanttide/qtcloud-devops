# ROADMAP

## v0.5.0:

- 取消 `release retire` 命令（保留 `code retire` — 子模块退役，不属 release 流程）
  - `main.rs`: 删除 `ReleaseAction::Retire` 变体及 match 分支
  - `commands/release.rs`: 删除 `retire()` 函数及相关测试（test_retire_*）
- 取消release的状态。
  - 领域模型（ReleaseStatus、ReleaseRecord、TransitionError）移到 `packages/toolkit`
  - 删除 `model/release.rs` 及 `commands/release.rs` 中的状态相关代码
