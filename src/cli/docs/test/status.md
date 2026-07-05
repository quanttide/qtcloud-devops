# test 命令设计

## 定位

运行测试、收集覆盖率，以及按 scope 输出测试结果。对应 DevOps 生命周期中的 **Test** 阶段。

## 命令

```
qtcloud-devops test status       按 scope 查看测试状态（读缓存，不运行测试）
qtcloud-devops test run          运行测试 + 覆盖率
qtcloud-devops test clean        清理缓存的测试结果
```

## 与 contract 模块的关系

| 来源 | 字段 | 用途 |
|------|------|------|
| `contract.stages.test.threshold` | `f64` | 全局默认覆盖率阈值（默认 70.0） |
| `scope.test_threshold` | `Option<f64>` | scope 级阈值覆盖 |
| `contract::scope_test_threshold()` | `f64` | scope 有值用 scope，无值用全局 |

## test status — 读取缓存的测试摘要

`test status` 只读缓存文件 `.quanttide/devops/test-summary.json`，**不运行测试**。测试结果由 `test run` 生成并缓存。

### 输出示例

```
测试状态
────────────────────────────────────────────────
  [cli]         Rust
    测试数:       42 ✅ 全部通过
    覆盖率:       85.3% ✅（阈值 80%）

  [studio]      Dart
    测试数:       18 ✅ 全部通过
    覆盖率:       92.1% ✅（阈值 90%）

  [provider]    Go
    测试数:       7 ✅ 全部通过
    覆盖率:       未检测到覆盖率报告
```

## test run — 运行测试 + 覆盖率

### 执行逻辑

1. 加载契约，确定 scope 列表
2. 若处于 scope 子目录中，仅运行该 scope
3. 对每个 scope：
   - 先运行覆盖率命令（Rust 用 `cargo llvm-cov`，自带测试；Python 用 `coverage xml` + 单独 `python -m pytest`）
   - 覆盖率命令失败时回退到仅运行测试
4. 运行测试后解析输出生成 `TestSummary`，缓存到 `.quanttide/devops/test-summary.json`

### 测试覆盖率命令

| Language | 命令 | 是否包含测试 |
|----------|------|------------|
| `Rust` | `cargo llvm-cov --lcov --output-path target/coverage/lcov.info` | ✅ 是（`cargo llvm-cov` 自带测试运行） |
| `Python` | `coverage xml` | ❌ 否，需先单独运行 `python -m pytest` |
| `Go` | `go tool cover -html=coverage.out -o coverage.html` | ❌ 否 |
| `Dart` | `flutter test --coverage` | ✅ 是 |
| `TypeScript` | `npx nyc --reporter=lcov npm test` | ✅ 是 |
| `Unknown` | 跳过覆盖率 | — |

### 测试命令

| Language | 命令 |
|----------|------|
| `Rust` | `cargo test` |
| `Python` | `python -m pytest` |
| `Go` | `go test ./...` |
| `Dart` | `flutter test` |
| `TypeScript` | `npm test` |
| `Unknown` | 跳过 |

### 输出示例

```
  运行测试...
  生成覆盖率 (cargo)...
  ✅ 覆盖率已更新
  ✅ 测试通过
```

## test clean — 清理缓存

删除 `.quanttide/devops/test-summary.json`，下次 `test status` 将显示缺省值。

## 覆盖率阈值优先级

```
scope.test_threshold? → Some → 使用 scope 级（如 90%）
                      → None → 使用 stages.test.threshold（全局默认 70%）
```

## 覆盖率解析

- Rust: 解析 `target/coverage/lcov.info`（DA 行记录 → 行命中率）
- Python: 解析 coverage 生成的 `cobertura.xml`（`line-rate` 属性）

足够做门禁检查，但不适合精确覆盖率分析（不按分支或函数统计）。

## 测试结果解析

解析 `cargo test` 的输出行：

```
test result: ok. 10 passed; 0 failed; 2 ignored; 0 measured; 12 filtered out
```

按 `;` 分割后取每个片段末尾的 `(数字, kind)` 对。不依赖固定位置——容错性好，测试框架输出微调也不崩。

## 公共 API

```rust
pub fn status(repo_path: &Path, c: &Contract)     // test status（读缓存）
pub fn status_to(writer: &mut impl Write, repo_path: &Path, c: &Contract) -> io::Result<()>
pub fn run(repo_path: &Path) -> Result<(), String> // test run（执行）
pub fn clear_cache(dir: &Path)                     // test clean
```

## 参考

- 依赖模块：`contract::{load, detect_by_files, resolve_language}`
