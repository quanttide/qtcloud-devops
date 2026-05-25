# TODO — v0.4.0

## 发布流程重构

### stage 关联预发布

- [ ] `stage` 改为推送 rc tag（版本号含 `-rc.N` 后缀）
  - 文件：`src/commands/release.rs` `stage()` 函数
  - 当前行为：只校验版本号 + 存 journal
  - 改为：校验 `-rc.N` 后缀 → `create_tag()` + `push_tag()` → 触发 CI
  - 版本号不含 `-rc.N` 时拒绝
- [ ] CI 新增 rc tag 构建 workflow
  - 文件：`.github/workflows/build-rc.yml`
  - 触发：`push: tags: ['cli/*-rc.*']`
  - 行为：构建 + 测试，不发布

### publish 不再打 tag

- [ ] `publish` 不做 tag，只创建 GitHub Release + 发布注册源
  - 文件：`src/commands/release.rs` `publish()` 函数
  - 移除 `create_tag()` 和 `push_tag()` 调用
  - 前置条件：对应 rc tag 已存在（`git tag -l` 验证）
- [ ] 验证：`publish -v cli/v0.3.2` 时检查 `cli/v0.3.2-rc.*` tag 存在

### 移除 cancel

- [ ] 删除 `Commands::Cancel` 枚举变体（`src/main.rs`）
- [ ] 删除 `cancel()` 函数（`src/commands/release.rs`）
- [ ] 删除 `cancel` 相关测试
- [ ] 更新文档和 help 文本
- [ ] publish 前加二次确认（替代 cancel 的保护作用）

## 发布目标支持

- [ ] **pub.dev 发布集成**：release 命令支持发布到 pub.dev
  - 调用 `dart pub publish`
- [ ] **发布目标抽象**：从 PyPI/pub.dev 提取 `PublishTarget` trait
  - `trait PublishTarget { fn publish(...) -> Result<()> }`
  - 实现：`PypiTarget`、`PubDevTarget`、`CratesTarget`
