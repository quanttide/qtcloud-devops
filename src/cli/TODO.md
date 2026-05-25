# TODO — v0.4.0

## 发布流程重构

### stage 关联预发布

- [ ] `stage` 改为推送 tag（仅限预发布版本）
  - 文件：`src/commands/release.rs` `stage()` 函数
  - 当前行为：只校验版本号 + 存 journal
  - 改为：校验含 semver 预发布后缀（`-rc.N`、`-alpha.N`、`-beta.N` 等）→ `create_tag()` + `push_tag()` → 触发 CI
  - 版本号不含预发布后缀时拒绝（正式版不走 stage）
- [ ] CI 新增 rc tag 构建 workflow
  - 文件：`.github/workflows/build-rc.yml`
  - 触发：`push: tags: ['cli/*-rc.*']`
  - 行为：构建 + 测试，不发布

### publish 打正式 tag

- [ ] `publish` 打正式 tag + GitHub Release + 注册源（当前行为不变）
  - 文件：`src/commands/release.rs` `publish()` 函数
  - 当前已实现：`create_tag()` + `push_tag()` + `create_release()`
- [ ] 前置条件：对应预发布 tag 已存在（验证 `cli/v0.3.2-*` 存在才能 publish `cli/v0.3.2`）

### 移除 cancel

- [ ] 删除 `Commands::Cancel` 枚举变体（`src/main.rs`）
- [ ] 删除 `cancel()` 函数（`src/commands/release.rs`）
- [ ] 删除 `cancel` 相关测试
- [ ] 更新文档和 help 文本
- [ ] publish 前加二次确认（替代 cancel 的保护作用）

## 发布渠道支持

- [ ] **pub.dev 发布集成**：release 命令支持发布到 pub.dev
  - 调用 `dart pub publish`
- [ ] **发布渠道抽象**：从 PyPI/pub.dev 提取 `Publisher` trait
  - `trait Publisher { fn publish(&self, version: &str, artifact_path: &Path) -> Result<()>; }`
  - 实现：`PypiPublisher`、`PubDevPublisher`、`CratesPublisher`
