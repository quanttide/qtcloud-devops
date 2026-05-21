"""DevOps 契约加载与查询示例。

用法:
    python examples/contract/contract.py <contract.yaml>
"""

from pathlib import Path
from typing import Any

import yaml


def load_contract(path: Path | str) -> dict[str, Any]:
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"契约文件不存在: {path}")
    with open(path, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    if not data:
        raise ValueError(f"契约文件为空: {path}")
    return data


def find_checks_for_action(contract: dict[str, Any], action: str) -> list[dict[str, Any]]:
    checks = contract.get("checks", [])
    return [c for c in checks if c.get("before") == action]


def resolve_rules(contract: dict[str, Any], rule_ids: list[str]) -> list[dict[str, Any]]:
    rules_map = {r["id"]: r for r in contract.get("contracts", [])}
    return [rules_map[rid] for rid in rule_ids if rid in rules_map]


if __name__ == "__main__":
    import sys
    path = sys.argv[1]
    contract = load_contract(path)
    print(f"约束: {len(contract.get('contracts', []))} 条")
    print(f"边界: {len(contract.get('boundaries', []))} 条")
    print(f"检查: {len(contract.get('checks', []))} 条")
