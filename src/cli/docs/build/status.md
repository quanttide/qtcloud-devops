# build status 命令设计

## 定位

按 scope 检查构建状态，只读模式，不触发构建。对应 DevOps 生命周期中的 **Build** 阶段。

## 命令

```
qtcloud-devops build status      按 scope 查看构建状态
```

## 与 contract 模块的关系

全过程依赖 `contract::load()`：

```
contract.yaml → Contract
                   ├── scopes[i].dir          → 确定检查哪个目录
                   ├── scopes[i].language      → 决定语法校验命令
                   ├── scopes[i].build_tool    → 决定校验工具
                   ├── platforms.artifact_registry → 显示制品库
                   ├── stages.release.changelog → 显示 CHANGELOG 路径
                   └── stages.test.threshold   → 显示测试阈值
```

## 三路检查

`build status` 不做本机构建。真正的构建由 CI 完成，`build status` 展示构建管线的整体状态——从三个维度覆盖：

| 检查 | 功能 | 数据来源 | 回答的问题 |
|------|------|---------|-----------|
| **CI 状态** | 展示 CI 最近一次构建结果 | `gh run list --workflow <name>` | "CI 上构建通过了吗？" |
| **语法校验** | 本地快速检查能否编译 | `cargo check` / `uv check` 等 | "本地代码有语法/类型错误吗？" |
| **版本一致** | tag 版本 vs 配置文件版本 | `contract::version_status()` | "待发布的版本号对齐了吗？" |

三路检查回答了三个不同的问题，合在一起才是完整的"构建状态"。

### CI 状态查询

按 scope 查对应 workflow，workflow 名称来源：

| 来源 | 优先级 | 示例 |
|------|--------|------|
| `scope.ci_workflow` | 高 | `ci_workflow: my-pipeline` → `--workflow my-pipeline` |
| 约定 `build-{scope}` | 低 | scope `cli` → `--workflow build-cli` |

匹配 `.github/workflows/build-cli.yml` 文件。workflow 不存在时输出 `⚠ 无 CI 运行记录`。

### 语法校验命令表

| Language | 命令 |
|----------|------|
| `Rust` | `cargo check` |
| `Python` | `uv check` |
| `Go` | `go vet` |
| `Dart` | `dart analyze` |
| `TypeScript` | `tsc --noEmit` 或 `npm run lint` |
| `Unknown` | 跳过，输出 `⚠ 语言未知，跳过语法校验` |

### 输出示例

```
构建状态
────────────────────────────────────────────────
  [cli]         Rust
    CI:         ✅ 通过 (main #42)
    syntax:     ✅ cargo check 通过
    version:    ✅ 0.6.1（一致）
    registry:   crates.io
    changelog:  CHANGELOG.md

  [studio]      Dart
    CI:         ✅ 通过 (main #18)
    syntax:     ⚠ dart analyze 有 3 个 warning
    version:    ✅ 0.1.3（一致）
    registry:   pub.dev
    changelog:  src/studio/CHANGELOG.md

  工作区:       ✅ 干净
```

## 关键设计

### 为什么不做本机构建

| 理由 | 说明 |
|------|------|
| **CI 才是真相** | 本地编译通过不代表 CI 能过（环境差异、依赖版本）。CI 的构建结果才是门禁标准。 |
| **耗时不对称** | `cargo check` 秒级，`cargo build` 分钟级。`status` 是只读诊断命令，不应让用户等。 |
| **只读语义** | `status` 不修改文件系统。`cargo build` 会产生构建产物，`cargo check` 只读。 |
| **各语言差异大** | Rust 有 `cargo check`，Go 有 `go vet`，但 Python/JS 没有等价的"编译但不产生产物"命令。强行统一为"构建"会破坏一致性。 |

### scope 复用 contract 模块

根 scope（无 `contract.yaml`）和命名 scope 都走同一套 `contract::version_status()` 接口，不重复实现。

### 错误反馈

- CI 状态查询失败（gh 未安装或无网络）：输出 `⚠ gh CLI 未安装`，不 panic
- 语法校验命令不存在：输出 `⚠ {tool} 未安装`，不阻断
- scope 目录不存在：输出 `⚠ 目录不存在`，跳过该 scope

## 公共 API

```rust
pub fn status(repo_path: &Path)
pub fn status_to(writer: &mut impl Write, repo_path: &Path) -> io::Result<()>
pub fn resolve_workflow(scope: &str, ci_workflow: Option<&str>) -> String
```

- `src/build.rs` 中的纯函数 `resolve_workflow` 处理 workflow 名称解析逻辑
- `status_to` 接受任意 writer，方便测试
- `status` 是 `status_to` 的 stdout 封装

## 参考

- 依赖模块：`contract::{load, version_status}`
