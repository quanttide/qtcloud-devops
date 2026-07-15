# 代码质量对比：Toolkit vs CLI

对比 `quanttide-devops-toolkit/packages/rust/`（简称 Toolkit）和 `qtcloud-devops/src/cli/`（简称 CLI）的代码质量。两者都是 Rust crate，定位不同：Toolkit 是纯库，CLI 是混合 lib + bin。

## 概览

| 维度 | Toolkit | CLI |
|------|---------|-----|
| 定位 | 纯库（library） | 混合 lib + bin |
| 版本 | 0.3.3 | 0.10.0 |
| Edition | 2024 | 2021 |
| 模块数 | 3 个顶级模块 | 9 个顶级模块 + 4 个子模块 |
| 估算代码行 | ~2,100 | ~5,500+ |
| 测试行占比 | ~45% | ~40–50% |
| `unsafe` 数量 | 1 | 0 |

## 1. 架构与模块化

### Toolkit — 高内聚分层

```
contract/  → model 层（契约数据结构 + 反序列化）
source/     → 事实源读取（data access）
stage/      → 生命周期状态枚举
```

每层职责单一。`contract` 不碰文件系统，`source` 不定义模型逻辑，`stage` 只定义状态—行为和输出在 CLI 侧。

### CLI — 功能驱动

```
src/
  main.rs     → clap 定义 + dispatch
  lib.rs      → 模块声明
  contract.rs → 适配层（薄封装 Toolkit）
  build.rs    → 构建状态 + CI 查询
  test.rs     → 测试运行 + 覆盖率解析 + 审计
  code.rs     → 子模块同步 + 代码审计
  plan.rs     → ROADMAP 管理 + LLM 审计
  git.rs      → 子模块扫描
  release/    → 发布子模块（audit/detect/publish/status）
  source/     → 系统诊断 + changelog 生成
  python.rs   → PyO3 绑定
```

**问题**：`plan.rs`（~1,400 行）和 `test.rs`（~1,400 行）过长，同时负责解析、校验、执行、格式化输出等多项职责，建议拆分为子模块（参考 `release/` 的结构）。

## 2. 错误处理

### Toolkit — 手写，完整但样板多

手动实现 `Display + Error + From`，包括中文错误消息和 source 链。每个模块重复一套。

```rust
impl fmt::Display for ContractError { ... }
impl std::error::Error for ContractError { ... }
impl From<io::Error> for ContractError { ... }
```

测试覆盖所有变体的 `to_string()` 和 `source()`。

### CLI — thiserror 优先，但不一致

`source/changelog.rs` 用 `thiserror` 派生：

```rust
#[derive(Debug, thiserror::Error)]
pub enum ChangelogError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    ...
}
```

但 `plan.rs` 等模块仍手写。风格不统一应当收敛。

**建议**：CLI 统一用 `thiserror`，Toolkit 可跟进引入。

## 3. 测试质量

### 共同优点

- 单测覆盖率极高，几乎每个函数有对应测试
- 使用 `tempfile` 隔离文件系统，无副作用
- 辅助函数复用（`git_init()`、`git_commit()` 在各模块重复）
- 测试命名清晰，覆盖正常、空、异常边界

### Toolkit 特色

- **doc test**：API 文档注释中嵌入 `assert!`，即文档即测试
- 集成测试 + 模块内 `#[cfg(test)]` 均齐全
- 自定义反序列化场景覆盖完整（Language/BuildTool 别名映射）

### CLI 特色

- 集成测试更复杂（`tests/cli.rs`、`tests/code.rs`、`tests/release.rs`）
- `release/detect.rs` 的标签排序测试详尽（~570 行）
- `git.rs` 的子模块状态检测覆盖 Detached/Orphaned/Ahead/Behind/Dirty 各分支（~566 行）

### CLI 不足

- 部分测试断言过于宽泛（`is_ok() || is_err()`）
- 部分测试缺少失败时的上下文信息（裸 `.unwrap()`）

## 4. 安全与代码实践

### unsafe

| 位置 | 说明 |
|------|------|
| Toolkit `changelog.rs` | `transmute<'static>` 延长 `parse_changelog::parse()` 返回的引用生命周期 |
| CLI | 无 |

Toolkit 的 `unsafe` 有详细 Safety 注释，且正确性可保证（`raw` 和 `inner` 始终一起移动）。但本质上可以用 `self_cell`、`ouroboros` 或全 owned 方案消除。

CLI 零 `unsafe`。

### 废弃代码

Toolkit 的 `detect_language` 已标注 `#[deprecated]`，说明替代方案 `detect_languages`。这是良好实践。

### 外部命令依赖

| 方面 | Toolkit | CLI |
|------|---------|-----|
| git 操作 | 纯 `gix` | `gix` + `git2` + `git CLI` |
| 系统命令 | 0 | 多处（`gh`、`cargo`、`python`、`node` 等） |
| LLM 调用 | 仅 prompt 构建 | 通过 `quanttide-agent` 真实调用 |

CLI 混合使用三套 git 工具有现实原因（git2 的 credential callback 问题），但增加了维护成本。

## 5. 代码可读性

### Toolkit

- 函数体积小（10–30 行），少有长函数
- Rust Edition 2024 语法（let-chains）
- `// ── section border ──` 分隔线辅助定位

### CLI

- 存在长函数：`parse_roadmap_str`（~70 行）、`plan_audit`（~52 行）
- `main.rs` dispatch 简洁（每函数 3–10 行）
- 输出格式统一（`"-".repeat(50)` 分隔 + emoji 图标）

## 6. 依赖版本差异

| 依赖 | Toolkit | CLI | 差异 |
|------|---------|-----|------|
|`gix` | 0.69 | 0.66 | CLI 落后 |
|`jiff` | 0.2 | 0.1 | CLI 落后 |
|`thiserror` | 无 | 2 | Toolkit 可引入 |

CLI 是 Toolkit 的消费者（`quanttide-devops = "0.3.1"`），但直接依赖的 `gix` 和 `jiff` 版本反而落后于 Toolkit。**建议同步升级**以避免潜在的 semver 冲突。

## 7. 综合评价

| 维度 | Toolkit | CLI |
|------|---------|-----|
| 架构清晰度 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| 错误处理 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| 测试质量 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| 代码简洁度 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| 安全（无 unsafe）| ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| API 文档 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| 功能完整性 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| 版本管理 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |

### Toolkit 主要优点

1. 架构干净，纯 model/access/state 分离
2. doc test 丰富，API 文档即测试
3. 无外部系统命令调用，易于测试和复用
4. `TagSource` trait 抽象设计良好，方便 mock
5. `#[deprecated]` 标注管理得当

### Toolkit 改进空间

1. 引入 `thiserror` 减少错误处理样板
2. 消除 `unsafe`（尽管已有充分 justification）
3. 可为 `Changelog` 考虑 `self_cell` 替代 transmute

### CLI 主要优点

1. 功能全面：构建/测试/发布/代码审计/LLM 集成一应俱全
2. 用户体验完整：美观输出、交互式确认、干运行模式
3. 发布工作流健壮：含 rollback 机制（`rollback_tag`、`delete_release`）
4. Python 绑定（`pyo3`）提供跨语言调用能力

### CLI 改进空间

1. **`plan.rs` 和 `test.rs` 拆分子模块**（参考 `release/` 模式）
2. **同步升级 `gix` 到 0.69、`jiff` 到 0.2**
3. **长函数拆分**：`parse_roadmap_str`、`audit` 等
4. **统一错误处理风格**：全部使用 `thiserror`
5. **`repo_path()` 隐式依赖 cwd**：可考虑显式通过参数传入
