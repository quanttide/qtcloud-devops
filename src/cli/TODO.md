# TODO — v0.4.x

## P0 — stage 预发布 + CI workflow

### stage 改为推送 tag

- [ ] `stage` 改为推送 tag（仅限含 semver 预发布后缀的版本）
  - 文件：`src/commands/release.rs` `stage()` 函数
  - 当前：校验版本号 + 存 journal
  - 改为：校验预发布后缀 → `create_tag()` + `push_tag()` → 触发 CI
  - journal 保留写入（审计用途），但 publish 不再依赖 journal 做校验
  - 版本号不含预发布后缀时拒绝

### 预发布构建 CI

- [ ] 新增 `.github/workflows/build-prerelease.yml`
  - 触发：`push: tags: ['cli/*-*']`（匹配所有含预发布后缀的 tag）
  - 行为：构建 + 测试，不发布
  - publish 时验证该 workflow 最后一次执行结果为 success

## P1 — publish 前置条件

- [ ] `publish` 验证对应预发布 tag 存在（`git tag -l 'cli/v0.3.2-*'` 有结果）
- [ ] `publish` 验证该 tag 触发的 CI 最后一次运行为 success
  - 方式：`gh run list --workflow build-prerelease.yml --branch cli/v0.3.2-rc.1 --json conclusion` 检查最新一次为 success
  - `gh` 已安装，不需要额外依赖
  - CI 未通过时拒绝发布
  - `gh run list` 因网络问题失败时，降级为允许发布，但提示"无法验证 CI 状态，请手动确认"

## P2 — 移除 cancel

- [ ] 删除 `Commands::Cancel`（`src/main.rs`）
- [ ] 删除 `cancel()` 函数 + 相关测试（`src/commands/release.rs`）
- [ ] 更新文档
- 二次确认已有 `confirm_release()` 实现，不需要额外改动

## P3 — pub.dev 发布集成

- [ ] `publish --registry <name>` 指定发布目标
  - 命名：`Registry` 枚举，variant 对齐官方注册源名（`PyPI`、`PubDev` 等）
  - `publish --registry pypi`（默认）
  - `publish --registry crates`
  - `publish --registry pub-dev`
- [ ] 先做具体实现，积累两个以上后再考虑提取 trait
