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

## P1 — publish 前置条件

- [x] `publish` 验证对应预发布 tag 存在（新增 `has_prerelease_tag()` 函数）

## P2 — 移除 cancel

- [x] 删除 `Commands::Cancel`（`src/main.rs`）
- [x] 删除 `cancel()` 函数 + 相关测试（`src/commands/release.rs`）
- [x] 更新文档
- 二次确认已有 `confirm_release()` 实现，不需要额外改动

## P3 — pub.dev 发布集成

- [ ] `publish --registry <name>` 指定发布目标
  - 命名：`Registry` 枚举，variant 对齐官方注册源名（`PyPI`、`PubDev` 等）
  - `publish --registry pypi`（默认）
  - `publish --registry crates`
  - `publish --registry pub-dev`
- [ ] 先做具体实现，积累两个以上后再考虑提取 trait
