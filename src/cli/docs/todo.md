# TODO — v0.5.0

## P0: 发布模型移入 toolkit

- [ ] 在 `packages/toolkit/packages/rust/src/` 创建 `lib.rs` + `release.rs`
  - 从 `model/release.rs` 提取：`ReleaseStatus`、`ReleaseRecord`、`TransitionError`
  - 去掉 serde derive（除非 toolkit 已依赖 serde）
  - 去掉 `uuid` 依赖（`id: String` 改为调用方传入）
  - 去掉 `timestamp()` 函数（时间戳由调用方传入）
- [ ] 更新 `packages/toolkit/packages/rust/Cargo.toml`（添加必要依赖）

## P0: 删除 `model/release.rs`

- [ ] 删除 `src/model/release.rs`
- [ ] `src/model/mod.rs`: 删除 `pub mod release;`

## P0: 清理 `commands/release.rs`

- [ ] 删除 import: `FileStorage`、`ReleaseRecord`、`ReleaseStatus`、`Storage`、`TransitionError`
- [ ] `stage()`: 删除 FileStorage 读写、状态分支判断、`ReleaseRecord::new_staged`
- [ ] `publish()`: 删除 FileStorage 读写、状态判断、`record.status = ReleaseStatus::Published`
- [ ] 删除 `release_status()` 函数
- [ ] 删除对应测试: `test_stage_*`（除 stage 幂等外）、`test_publish_not_staged`、全部 `test_release_status_*`

## P1: 清理 `main.rs`

- [ ] 删除 `ReleaseAction::Retire` 变体
- [ ] 删除 `ReleaseAction::Status` 变体
- [ ] 删除 match 中对应的处理分支

## 验证

- [ ] `cargo build` 通过
- [ ] `cargo test` 全部通过（存量 175 测试）
- [ ] `cargo test --test release` 通过
