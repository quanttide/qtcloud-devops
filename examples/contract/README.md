# Contract 示例

DevOps 契约加载与查询的参考实现。

## 用法

```bash
# 需要安装 pyyaml
uv add pyyaml

# 加载并展示契约
python contract.py ../../tests/fixtures/contract.yaml
```

## API

| 函数 | 说明 |
|------|------|
| `load_contract(path)` | 加载 YAML 契约文件 |
| `find_checks_for_action(contract, action)` | 查找指定操作的预检查 |
| `resolve_rules(contract, rule_ids)` | 根据规则 ID 解析规则详情 |

## 参考

契约文件格式见 [tests/fixtures/contract.yaml](../../tests/fixtures/contract.yaml)。
