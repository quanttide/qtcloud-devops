# TODO — v0.5.0

## P0: 发布模型移入 toolkit

- [x] 在 `packages/toolkit/packages/rust/src/` 创建 `lib.rs` + `release.rs`
  - 从 `model/release.rs` 提取：`ReleaseStatus`、`ReleaseRecord`、`TransitionError`
  - 去掉 serde derive（除非 toolkit 已依赖 serde）
  - 去掉 `uuid` 依赖（`id: String` 改为调用方传入）
  - 去掉 `timestamp()` 函数（时间戳由调用方传入）
- [x] 更新 `packages/toolkit/packages/rust/Cargo.toml`（添加 `[lib]` 声明，纯标准库零依赖）

## P0: 删除 `model/release.rs`

- [x] 删除 `src/model/release.rs`
- [x] `src/model/mod.rs`: 删除 `pub mod release;`

## P0: 清理 `commands/release.rs`

- [x] 删除 import: `FileStorage`、`ReleaseRecord`、`ReleaseStatus`、`Storage`、`TransitionError`
- [x] `stage()`: 删除 FileStorage 读写、状态分支判断、`ReleaseRecord::new_staged`
- [x] `publish()`: 删除 FileStorage 读写、状态判断、`record.status = ReleaseStatus::Published`
- [x] 删除 `retire()`、`release_status()` 函数
- [x] 删除对应测试; 清理 `tests/release.rs`（集成测试）和 `tests/cli.rs`（CLI 测试）

## P1: 清理 `main.rs`

- [x] 删除 `ReleaseAction::Retire` 变体及 match 分支
- [x] 删除 `ReleaseAction::Status` 变体及 match 分支

## 验证

- [x] `cargo build` 通过
- [x] `cargo test` 全部通过（114 单元 + 6 main + 12 cli + 13 code + 6 release = 151 测试）
- [x] `cargo test --test release` 通过
