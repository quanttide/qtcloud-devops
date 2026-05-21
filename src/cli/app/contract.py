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
