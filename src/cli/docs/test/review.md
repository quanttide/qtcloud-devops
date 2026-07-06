# test review — 质量审查

## 定位

扫描源码和测试，评估测试质量。这不是跑测试，而是**对测试的测试**——回答"我们的测试够好吗？"

自举：本工具的质量审查由 `test review` 自身定义和执行，门禁配置在 `contract.yaml`。

## 扫描维度

| 维度 | 测量方法 | 数据来源 |
|------|----------|----------|
| **测试数量** | 每个模块的 `#[test]` 函数数 | 源码扫描 |
| **函数覆盖率** | 有 `#[test]` 引用的 `pub fn` 比例 | 源码扫描 |
| **错误变体覆盖** | `enum XxxError` 的每个变体是否被测试引用 | 源码扫描 |
| **纯函数覆盖** | `fn`（非 I/O）是否有对应 `test_*` 函数 | 源码扫描 |
| **行覆盖率** | lcov/cobertura 解析 | 缓存文件 |
| **门禁达标** | 以上各维度 vs 契约定义阈值 | contract.yaml |

## 自举设计

`contract.yaml` 扩展（暂定，待与 `quanttide-devops` 库协调）：

```yaml
stages:
  test:
    command: cargo test
    threshold: 70          # 行覆盖率阈值
    quality_gates:
      min_test_count: 0      # 模块最低测试数（0=不检查）
      error_variant_coverage: 0.5  # 错误变体至少 50% 有测
      pure_fn_coverage: 0.3       # 纯函数至少 30% 有测
```

门禁不达标时 `test review` 退出码非零（可被 CI 捕获）。

## 命令

```
qtcloud-devops test review              审查当前 scope
qtcloud-devops test review --all        审查所有 scope
qtcloud-devops test review --verbose    展示每个未覆盖函数
```

## 输出示例

```
测试质量审查
────────────────────────────────────────────────
  [cli]         Rust
    测试函数:     42
    函数覆盖率:   68% (34/50)  ⚠ (门禁: 70%)
    错误变体覆盖: 12/15       ⚠ (门禁: 80%)
      未覆盖: Io, NotModified
    行覆盖率:     85.3%       ✅ (门禁: 70%)

  [studio]      Dart
    测试函数:     18
    函数覆盖率:   90% (18/20)  ✅ (门禁: 70%)
    行覆盖率:     92.1%       ✅ (门禁: 90%)

  总计: 2/3 门禁通过
```

## 与 `test status` 的区别

| | `test status` | `test review` |
|--|--------------|---------------|
| 是否运行测试 | ❌ 读缓存 | ❌ 纯静态扫描 |
| 是否解析覆盖率 | ✅ | ✅（复用缓存） |
| 是否扫描源码 | ❌ | ✅（计数/分析） |
| 是否参考门禁 | ✅（仅阈值） | ✅（质量门禁矩阵） |
| 退出码 | 0 | 门禁达标 0，否则 1 |

## 实现思路

1. `collect_metrics(scope_dir, lang)` → 扫描源码，收集各维度数据
2. `load_gates(contract, scope)` → 读取契约门禁配置
3. `evaluate(metrics, gates)` → 逐条比较，生成报告项
4. 打印报告，门禁不达标时返回非零退出码
