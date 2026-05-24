# AGENTS

## ROADMAP

- 只记录**未完成**的事项，已完成的及时删除
- 按 P0-P3 优先级分组
- 基本假设放在末尾，不随版本迭代删除
- 新增需求先定优先级再放入对应分组

## 开发规范

### 发布工作纪律

**工作纪律 1：先提交，后发布**

```
❌ 错误顺序：
   edit Cargo.toml → cargo build → stage/publish → git commit
   → tag 指向的 Cargo.toml 版本与 tag 版本不一致，CI validate 失败

✅ 正确顺序：
   edit Cargo.toml + CHANGELOG → git commit → cargo build → stage/publish
   → tag 指向已提交的代码，版本一致
```

**工作纪律 2：预发布版本（rc）只用于 CI 测试**

- rc 版本（`v0.3.0-rc.N`）用于验证 CI 构建与发布流程
- 每次 rc 只修复一个 CI 问题，失败后递增 rc 序号，不删除已有 release
- rc 版本通过后，正式版（`v0.3.0`）走相同流程

**工作纪律 3：测试与发布分离**

- 单元测试在 push/PR 时由 test CI 执行（待建立）
- build/publish CI 不运行测试，避免环境问题阻塞发布

## 提交消息

- `feat:` — 新功能
- `chore:` — 版本号变更、配置更新
- `docs:` — 文档更新
- `fix:` — 修 bug
- `test:` — 测试

## CLI 设计规则

### `code` 子命令行为

```
qtcloud-devops code status [path]                # 三路 commit 比对 + 聚合统计
qtcloud-devops code sync [name] [--repo path]    # 同步子模块指针到父仓库
qtcloud-devops code retire <name> [--repo path]  # 退役子模块
```

### 规则

- `status`：路径默认为当前目录 `.`
- `sync`：`name` 省略时同步全部子模块
- `retire`：`name` 为必填参数
- 所有命令由 Rust 直接实现，无 Python 封装层

### release 命令行为

```bash
qtcloud-devops stage -v <version>        # 标记版本，进入 Staged 状态
qtcloud-devops publish -v <version> [-y] # Staged → Published（标签 + GitHub Release）
qtcloud-devops cancel -v <version>       # Staged → Cancelled
qtcloud-devops retire -v <version>       # Published → Retired
```

### 规则

- 版本号格式：`vX.Y.Z` 或 `scope/vX.Y.Z`（如 `cli/v0.3.0`）
- scope 前缀用于多仓库场景，CI 通过 `startsWith(github.ref, 'refs/tags/scope/')` 过滤
- 发布流程：`stage` → `publish`（两步）。`stage` 只校验版本号，`publish` 执行 tag + GitHub Release
- 回滚：`create_tag` 失败无副作用；`push_tag` 失败删本地 tag；GitHub Release 失败删本地+远程 tag
- 仓库自动检测：从 `git remote get-url origin` 解析 GitHub 仓库名（`get_remote_repo()`）

## 测试

```sh
cargo test          # 全部 129 测试
cargo test --test release  # 仅 release 集成测试
cargo test --test code     # 仅 code 集成测试
```

## CI 工作流

| 工作流 | 触发 | 行为 |
|--------|------|------|
| `build-cli` | `release: [published]` + tag `cli/*` | 版本校验 → 三平台构建 → wheel 构建 |
| `publish-cli` | `workflow_run` (build-cli 成功) | publish-crate + publish-pypi（独立 job） |
