# code audit — 代码质量审计

## 定位

`code audit` 是 DevOps 生命周期的**门禁检查**，而非深度代码分析。它回答"过还是不过"，不回答"怎么改"。

### 与 qtcloud-code 的边界

| 层级 | 工具 | 做什么 | 判断标准 |
|------|------|--------|----------|
| **门禁** | `qtcloud-devops code audit` | 文本级统计（函数长度、TODO 密度、嵌套深度、import 数、文件长度、模块文档等） | 红/绿，CI 阻断 |
| **诊断** | `qtcloud-code review` | AST 级分析（圈复杂度、长函数、未用变量、missing tests） | 精确到行号和列，给出修复建议 |

- devops 不需要 parser——新指标的门槛是"能否在不引入 tree-sitter 的前提下实现"
- code 输出 `STATUS.md`，devops 读到就聚合展示，但不依赖它做门禁判定
- 两者独立发布、独立演进

## 架构

扫描过程分三层：

```
walk_scope_files()      → 遍历 contract.yaml 中所有 scope 的 dir，收集源码文件路径
scan_files()            → 批量扫描，返回 Vec<ScannedFile>
check_*() × 9          → 每个检查项产出 RuleResult { name, passed, details }
print_report()          → 汇总输出
```

核心数据结构：

| 类型 | 字段 | 说明 |
|------|------|------|
| `ScannedFile` | `path`, `lines`, `todos`, `imports`, `long_fns`, `missing_docs`, `max_nesting`, `high_complexity`, `has_mod_doc` | 一次扫描、多维度复用 |
| `RuleResult` | `name`, `passed`, `details` | 每个检查项自描述，统一渲染 |

## 扫描维度（9 项检查）

| 维度 | 测量方法 | 阈值 |
|------|----------|------|
| **Scope 目录完整性** | 对照 `contract.yaml` 的 scope 定义检查 `dir` 路径是否存在 | 全部存在 |
| **TODO/FIXME/HACK 密度** | 扫描源码文件的 `todo`/`fixme`/`hack` 标记位置，计算千行密度 | < 5‰ |
| **函数长度** | 基于 `fn`/`def`/`func` 定义行，计算到下一个函数定义之间的行数 | ≤ 40 行（超过 80 行标记"大幅超限"） |
| **API 文档覆盖率** | `pub fn` / `pub(crate) fn` 前 1-3 行的 `///` 注释存在性 | 全部有文档注释 |
| **结构复杂度** | 最大缩进嵌套深度、圈复杂度（基于分支关键字计数） | 嵌套 ≤ 4 层，圈复杂度 ≤ 10 |
| **导入数** | 单文件 `use`/`import` 声明计数 | ≤ 30 个 import |
| **文件长度** | 单文件总行数 | ≤ 500 行 |
| **模块文档** | 文件前 10 行是否存在 `//!` 声明 | 全部包含 `//!` |
| **语法检查** | 按语言执行外部 lint 命令 | 全部通过 |

## 语言支持

扫描的源码扩展名：`rs`, `py`, `go`, `ts`, `tsx`, `dart`, `js`, `jsx`。

函数定义检测按语言自动适配：

| 语言 | 检测模式 |
|------|----------|
| Rust | `fn ` / `pub fn ` / `pub(crate) fn ` |
| Python | `def ` |
| Go | `func ` |
| 其他 | 回退到 Rust 模式 |

语法检查支持的语言：

| 语言 | 命令 | 说明 |
|------|------|------|
| Rust | `cargo check --quiet` | 编译检查 |
| Python | `uv check` | Pyright 静态检查 |
| TypeScript | `npx tsc --noEmit` | 类型检查 |
| 其他 | 跳过 | — |

语言由 contract.yaml 自动检测，不支持的语言跳过语法检查（不计入失败）。

## 输出示例

```
代码审计
--------------------------------------------------
  ✅ Scope 目录: 全部 2 个 scope 存在
  ❌ TODO/FIXME 密度: 103 处, 密度 8.6‰（阈值 5‰）
  ❌ src/test.rs: `collect_error_variants` 71 行（超限）
  ❌ src/test.rs: `foo` 缺少文档注释
  ❌ src/plan.rs: 嵌套深度 8 层
  ❌ src/test.rs: `collect_error_variants` 圈复杂度 19
  ✅ 导入数: 全部文件 ≤ 30 个 import
  ❌ src/test.rs (1472 行)
  ❌ 32/34 文件缺少 //!（覆盖率 6%）
     cargo check: ✅
--------------------------------------------------
  ⚠ 3/9 项通过
```

## JSON 输出（--json）

以 JSON 格式输出审计结果，供 `plan todo-from-audit` 消费。

```bash
# 查看 JSON
qtcloud-devops code audit --json

# 管道到 plan 写入 TODO.md
qtcloud-devops code audit --json | qtcloud-devops plan todo-from-audit

# 两步操作（CI 场景）
qtcloud-devops code audit --json > audit.json
qtcloud-devops plan todo-from-audit < audit.json
```

JSON 结构：

```json
{
  "source": "code-audit",
  "source_label": "代码审计",
  "entries": [
    {
      "priority": "MUST",
      "items": [
        { "file": "src/test.rs", "detail": "圈复杂度 19" }
      ]
    }
  ]
}
```

### 优先级分级

`plan todo-from-audit` 按以下规则将检查项写入 TODO.md 的 `#### MUST` / `#### SHOULD` / `#### MAY` 子节：

| 优先级 | 条件 |
|--------|------|
| MUST | Scope 目录缺失、语法检查失败、函数大幅超限（>80 行）、结构复杂度超标 |
| SHOULD | 函数超限（>40 行）、API 文档缺失、导入数超标 |
| MAY | TODO/FIXME 密度、文件长度、模块文档缺失 |

多次运行幂等——已有的 `### 代码审计` 节会被整节替换，不会重复追加。

`code audit` 的每次运行都是对自身的间接检查——如果一个提交把审计代码本身改得更复杂，下次运行就会检出它自己。这在 AGENTS.md 中记为经验第 7 条：**"用工具扫自己比手动挑问题更系统"**。
