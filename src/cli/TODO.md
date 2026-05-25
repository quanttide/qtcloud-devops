# TODO — v0.4.x

## P0 — stage 预发布 + CI workflow

### stage 改为推送 tag

- [x] `stage` 改为推送 tag（仅限含 semver 预发布后缀的版本）
  - 文件：`src/commands/release.rs` `stage()` 函数
  - 新增 `is_prerelease()` 校验，拒绝正式版
  - 新增 `create_tag()` + `push_tag()`，推送 tag 触发 CI
  - journal 保留写入（审计用途）

### 预发布构建 CI

- [x] 新增 `.github/workflows/build-prerelease.yml`
  - 触发：`push: tags: ['cli/*-*']`（匹配所有含预发布后缀的 tag）
  - 行为：构建 + 测试，不发布
  - publish 时验证该 workflow 最后一次执行结果为 success

## P1 — publish 前置条件（已废弃，相关代码已删除）

## P2 — 移除 cancel

- [x] 删除 `Commands::Cancel`（`src/main.rs`）
- [x] 删除 `cancel()` 函数 + 相关测试（`src/commands/release.rs`）
- [x] 更新文档
- 二次确认已有 `confirm_release()` 实现，不需要额外改动

## P3 — pub.dev 发布集成

- [x] 新增 `Registry` 枚举（`PyPI`、`PubDev`、`Crates`）
- [x] `publish --registry <name>` 指定发布目标
  - registry 发布由 CI 处理，本地仅传递参数
