# AGENTS

## AI 操作原则

### 禁止随意删除文件

AI 不得在不读取文件内容的情况下删除文件。删除前必须：

1. 读取文件内容，确认用途
2. 判断文件是否属于项目代码、文档、配置
3. 如不确定，保留文件并向人询问

特别地，`docs/` 下所有文件优先视为项目文档，非代码文件不因"不认识"而删除。

`git add -A` 会捡到陌生文件，AI 应当在 commit 前 review 变更清单，而不是 commit 后再去 amend 清理。

## 开发规范

### 发布工作纪律

**工作纪律 1：AI 禁止直接 publish**

AI 的 git 操作止步于 `commit && push`。tag、stage、publish 由人执行（或经人确认后 AI 可执行 `stage`/`publish` 命令，但 tag 必须指向已提交的代码）。

```
❌ AI 不应直接运行：
   git tag cli/v0.3.0 && git push origin cli/v0.3.0
   qtcloud-devops publish -v cli/v0.3.0 -y

✅ AI 可以做的事：
   edit Cargo.toml + CHANGELOG → git commit && git push
   → 由人打 tag 或确认后 AI 执行 stage/publish
```

**工作纪律 2：发布前预验证**

push 之前，运行预验证脚本：

```bash
./scripts/preflight.sh
```

脚本内容应至少包含：

```bash
cargo build --release                    # 确认编译通过
cargo test                               # 确认测试通过
cargo publish --dry-run --registry crates-io  # 确认 crates.io 发布可行
maturin build --release --out dist       # 确认 wheel 构建可行
```

preflight 不通过不发布。

**工作纪律 3：最少预发布版本数**

一次 rc 应承载完整的 CI 验证（构建 + 发布），而不是分多次。需要验证的三件事：

1. **操作系统兼容性**：Linux / macOS / Windows
2. **注册源发布**：crates.io + PyPI
3. **版本元数据**：license、路径、CHANGELOG

如果一次 rc 因为某个问题失败，修复后应预期其余步骤不受影响（而不是出现新的问题）。出现新问题说明本地预验证不足。

**工作纪律 4：stage 关联预发布**

`stage` 只用于 rc 版本，不直接 stage 正式版。

```
✅ 正确流程：
   stage -v cli/v0.3.2-rc.1    ← 标记 rc
   # CI 验证 rc 通过
   publish -v cli/v0.3.2 -y     ← 正式发布

✅ rc 失败时：
   直接 stage 下一个，不 cancel（旧 tag 保留作为记录）
   stage -v cli/v0.3.2-rc.2     ← 递增 rc 序号

❌ 错误做法（跳过 rc）：
   stage -v cli/v0.3.2          ← stage 不应直接用于正式版
   publish -v cli/v0.3.2 -y     ← 应走 rc → publish 流程
```

**工作纪律 5：测试与发布分离**

- 单元测试在 push/PR 时由 test CI 执行（待建立）
- build/publish CI 不运行测试，避免环境问题阻塞发布

### 拒绝规则测试原则

每新增一条发布层面的拒绝规则，必须在三个层次各加一个测试：

1. **单元/辅助函数层** — 验证拒绝逻辑本身正确
2. **生产路径层**（`stage`/`publish` 直接调用） — 验证拒绝规则在真实流程中被执行
3. **CLI 子进程层**（端到端） — 验证用户看到的错误消息正确

测试方式：`.is_err()` + 检查错误消息关键字。只测 `.is_err()` 不检查消息会被视为不完整。

反面案例（踩过的坑）：`precheck_version_changelog` 有独立单元测试，但 `stage`/`publish` 没有测试"缺少 CHANGELOG 时应当拒绝"这个场景，导致该规则虽然存在但从未被执行。

### 重构原则

**1. 提取私函数不改测试** — 从 `scan()` 拆出 4 个私有辅助函数后，存量 13 个 `test_scan_*` 全部通过，零改动。私有函数通过 `pub fn` 入口被间接覆盖，证明提取正确。

**2. 纯逻辑函数值得单独的单元测试** — `determine_submodule_status` 有 6 个布尔条件产生 64 种组合，只靠 `scan()` 的集成测试无法穷举。提取后补了 7 个测试，发现了 1 个对优先级的认知错误（ahead+behind 同时存在时实际走 BehindRemote 而非 Clean）。纯函数测试成本低、收益高。

**3. 非纯函数不要单独测** — git 操作的私有函数（`scan_submodule_remote_state`、`check_parent_dirty`）如果单独测需要 mock repo，测试代码比生产代码还复杂。让集成测试间接覆盖就够了。

判断标准：**纯函数（无 I/O、无外部状态）→ 单元测试；编排/I/O → 集成测试覆盖。**

**4. 分级阈值比单阈值有用** — 30/50/80 三档（对应 MAY/SHOULD/MUST）比统一 30 行更区分优先级。开发者不需要"全是问题"的清单，只需要"先修什么"的指引。

**5. 测试样板是值得消灭的重复** — 测试里的 Submodule 构造器每个 10 行 × 7 次 = 70 行，用 `sm(name, status)` 辅助函数缩到 1 行。测试可读性不降反升，因为差异部分更突出了。

**6. Rust 辅助函数只能放在 inherent impl** — trait impl 里不能加非 trait 方法，必须放在 `impl StructName` 块。

**7. 用工具扫自己比手动挑问题更系统** — qtcloud-code 扫 qtcloud-devops-cli 发现的 3 MUST 全部消除，零遗漏。工具做筛选，人做判断。

## 提交消息

- `feat:` — 新功能
- `chore:` — 版本号变更、配置更新
- `docs:` — 文档更新
- `fix:` — 修 bug
- `test:` — 测试

## CLI 设计规则

### 模块结构

```
src/
├── code/       # 业务层：纯抽象，不暴露 git 概念
├── git/        # 事实源底层：所有 git 操作
└── release/    # 发布子领域：stage → publish
```

### `code` 命令

```bash
code sync [name]                # 同步组件（封装 fetch + push + pointer update）
code status [path] [--offline]  # 查看组件同步状态
```

- `sync`：`name` 省略时同步全部
- `status`：路径默认为当前目录 `.`

### `release` 命令

```bash
release publish -v <version> [--pre-release]  # 发布版本（--pre-release 跳过确认，校验版本后缀）
```

- 版本号格式：`vX.Y.Z` 或 `scope/vX.Y.Z`
- `stage` 只用于预发布版本（含 `-rc.N`、`-alpha.N` 等后缀）
- 回滚：`create_tag` 失败无副作用；`push_tag` 失败删本地 tag；GitHub Release 失败删本地+远程 tag

## CI 工作流

| 工作流 | 触发 | 行为 |
|--------|------|------|
| `build-cli` | `release: [published]` + tag `cli/*` | 版本校验 → 三平台构建 → wheel 构建 |
| `publish-cli` | `workflow_run` (build-cli 成功) | publish-crate + publish-pypi（独立 job） |
