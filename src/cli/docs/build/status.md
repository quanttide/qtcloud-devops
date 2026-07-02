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

## 每 scope 三路检查

| 检查项 | 实现 | 说明 |
|--------|------|------|
| CI 状态 | `gh run list --workflow <scope> --limit 1` | 查最近一次 CI 结果；gh 未安装时标记 `⚠` |
| 语法校验 | `cargo check` / `go vet` / `dart analyze` 等 | 按 `Language` 调对应命令 |
| 版本一致 | `contract::version_status()` | tag 版本 vs 配置文件版本 |

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

### scope 复用 contract 模块

根 scope（无 `contract.yaml`）和命名 scope 都走同一套 `contract::version_status()` + `contract::scope_release()` 接口，不重复实现。

### 错误反馈

- CI 状态查询失败（gh 未安装或无网络）：输出 `⚠ gh CLI 未安装`，不 panic
- 语法校验命令不存在：输出 `⚠ {tool} 未安装`，不阻断
- scope 目录不存在：输出 `⚠ 目录不存在`，跳过该 scope

### 不做的

- 不主动触发构建（CI 自动执行）
- 不做全量编译（`cargo check` 而非 `cargo build`）

## 实现步骤

### 第一步：新建 `src/build.rs`

从实验室 `examples/default/src/build.rs` 复制核心逻辑，核心函数签名：

```rust
pub fn status(repo_path: &Path)
```

内部流程：
1. `contract::load(repo_path)` 加载契约
2. 遍历 scopes（无 scope 时构造 root Scope）
3. 对每个 scope：查 CI → 语法校验 → 版本一致性检查
4. 显示工作区状态（`git status --porcelain`）

### 第二步：注册模块

```rust
// src/lib.rs
pub mod build;
```

### 第三步：注册 CLI 子命令

```rust
// src/main.rs
enum Commands {
    Build {
        #[command(subcommand)]
        action: BuildAction,
    },
}

enum BuildAction {
    Status,
}
```

## 参考

- 实验室原型：`examples/default/src/build.rs`（3 测试）
- 设计文档：`examples/default/docs/build.md`
- 蓝图来源：`data/roadmap/platform/build-command.md`
- 依赖模块：`contract::{load, version_status, scope_release, resolve_language}`
