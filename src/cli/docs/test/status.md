# test status 命令设计

## 定位

按 scope 输出测试结果和覆盖率，检查是否达到阈值。对应 DevOps 生命周期中的 **Test** 阶段。

## 命令

```
qtcloud-devops test status       按 scope 查看测试状态
```

## 与 contract 模块的关系

| 来源 | 字段 | 用途 |
|------|------|------|
| `contract.stages.test.threshold` | `f64` | 全局默认覆盖率阈值（默认 70.0） |
| `scope.test_threshold` | `Option<f64>` | scope 级阈值覆盖 |
| `contract::scope_test_threshold()` | `f64` | scope 有值用 scope，无值用全局 |

## 每 scope 两项检查

| 检查项 | 实现 | 说明 |
|--------|------|------|
| 测试结果 | 运行 `cargo test` 并解析输出 | 解析 `test result: N passed; M failed; K ignored` 格式 |
| 覆盖率 | 读取 `target/coverage/lcov.info` | 行命中率，与阈值比较 |

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

## 关键设计

### 阈值优先级

```
scope.test_threshold? → Some → 使用 scope 级（如 90%）
                      → None → 使用 stages.test.threshold（全局默认 70%）
```

### 覆盖率解析

按行命中率计算：

```
lcov.info 格式:
  DA:1,1       ← 第 1 行被命中
  DA:2,0       ← 第 2 行未被命中

覆盖率 = 命中行数 / 总行数 × 100%
```

足够做门禁检查，但不适合精确覆盖率分析（不按分支或函数统计）。

### 测试结果解析

解析 `cargo test` 的输出行：

```
test result: ok. 10 passed; 0 failed; 2 ignored; 0 measured; 12 filtered out
```

按 `;` 分割后取每个片段末尾的 `(数字, kind)` 对。不依赖固定位置——容错性好，测试框架输出微调也不崩。

## 不做的

- 不运行测试覆盖工具（如 `tarpaulin` / `kcov`），覆盖率报告由 CI 生成
- 不缓存测试结果，每次执行实时运行

## 实现步骤

### 第一步：新建 `src/test.rs`

从实验室 `examples/default/src/test.rs` 复制核心逻辑。核心函数签名：

```rust
pub fn status(repo_path: &Path, c: &contract::Contract)
```

内部流程：
1. 从 `contract::load_scopes()` 获取 scope 列表
2. 无 scope：检测语言，汇总测试，用全局 `c.stages.test.threshold`
3. 有 scope：遍历 scopes，用 `contract::scope_test_threshold()` 获取各 scope 阈值
4. 对每个 scope：运行测试 → 收集覆盖率 → 与阈值比较

### 第二步：注册模块

```rust
// src/lib.rs
pub mod test;
```

### 第三步：注册 CLI 子命令

```rust
// src/main.rs
enum Commands {
    Test {
        #[command(subcommand)]
        action: TestAction,
    },
}

enum TestAction {
    Status,
}
```

## 参考

- 实验室原型：`examples/default/src/test.rs`（6 测试）
- 设计文档：`examples/default/docs/test.md`
- 依赖模块：`contract::{load_scopes, scope_test_threshold, resolve_language, detect_by_files}`
