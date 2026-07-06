# code audit — 代码质量审计

## 定位

`code audit` 是 DevOps 生命周期的**门禁检查**，而非深度代码分析。它回答"过还是不过"，不回答"怎么改"。

### 与 qtcloud-code 的边界

| 层级 | 工具 | 做什么 | 判断标准 |
|------|------|--------|----------|
| **门禁** | `qtcloud-devops code audit` | 文本级统计（TODO 密度、unwrap 密度、文件长度、导入数） | 红/绿，CI 阻断 |
| **诊断** | `qtcloud-code review` | AST 级分析（圈复杂度、长函数、未用变量、missing tests） | 精确到行号和列，给出修复建议 |

- devops 不需要 parser——新指标的门槛是"能否在不引入 tree-sitter 的前提下实现"
- code 输出 `STATUS.md`，devops 读到就聚合展示，但不依赖它做门禁判定
- 两者独立发布、独立演进

## 扫描维度

| 维度 | 测量方法 | 阈值 |
|------|----------|------|
| **Scope 目录完整性** | 对照 `contract.yaml` 的 scope 定义检查 `dir` 路径是否存在 | 全部存在 |
| **TODO/FIXME/HACK 密度** | 扫描源码文件的 `todo`/`fixme`/`hack` 标记，计算千行密度 | < 5‰ |
| **unwrap/expect 密度** | 扫描源码文件的 `.unwrap()` / `.expect(` 调用，计算千行密度 | < 10‰ |
| **导入数** | 单文件超过阈值时报出路径和 import 数 | ≤ 30 个 import |
| **文件长度** | 单文件超过阈值时报出路径和行数 | ≤ 500 行 |
| **语法检查** | 按语言执行外部 lint 命令 | 全部通过 |

## 语言支持

扫描的源码扩展名：`rs`, `py`, `go`, `ts`, `tsx`, `dart`, `js`, `jsx`。TODO/FIXME、unwrap/expect、文件长度三项检查只统计这些扩展名。

语法检查支持的语言：

| 语言 | 命令 | 说明 |
|------|------|------|
| Rust | `cargo check --quiet` | 编译检查 |
| Python | `uv check` | Pyright 静态检查 |
| TypeScript | `npx tsc --noEmit` | 类型检查 |
| 其他 | 跳过 | — |

语言自动检测，不支持的语言跳过语法检查（不计入失败）。

## 输出示例

```
代码审计
--------------------------------------------------
  ✅ Scope 目录: 全部 3 个 scope 存在
  ✅ TODO/FIXME: 8 处, 密度 1.2‰
  ✅ unwrap/expect: 23 处, 密度 3.5‰
  ✅ 导入数: 全部文件 ≤ 30 个 import
  ✅ 文件长度: 全部文件 ≤ 500 行
     cargo check: ✅
  ✅ 语法检查: 通过
--------------------------------------------------
  ✅ 全部 6 项检查通过
```
