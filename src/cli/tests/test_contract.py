import pytest
from app.contract import load_contract, find_checks_for_action, resolve_rules


def test_load_contract(contract_path):
    contract = load_contract(contract_path)
    assert "contracts" in contract
    assert "checks" in contract
    assert "boundaries" in contract


def test_load_contract_not_found():
    with pytest.raises(FileNotFoundError):
        load_contract("/nonexistent/contract.yaml")


def test_load_contract_empty(tmp_path):
    empty_file = tmp_path / "empty.yaml"
    empty_file.write_text("")
    with pytest.raises(ValueError, match="契约文件为空"):
        load_contract(empty_file)


def test_find_checks_for_action(contract_path):
    contract = load_contract(contract_path)
    checks = find_checks_for_action(contract, "git tag")
    assert len(checks) == 1
    assert checks[0]["id"] == "pre-tag-check"


def test_find_checks_for_action_no_match(contract_path):
    contract = load_contract(contract_path)
    checks = find_checks_for_action(contract, "git nonexistent")
    assert len(checks) == 0


def test_resolve_rules(contract_path):
    contract = load_contract(contract_path)
    rules = resolve_rules(contract, ["release-changelog-required", "release-clean-working-tree"])
    assert len(rules) == 2
    assert rules[0]["id"] == "release-changelog-required"
    assert rules[1]["id"] == "release-clean-working-tree"


def test_resolve_rules_skip_missing(contract_path):
    contract = load_contract(contract_path)
    rules = resolve_rules(contract, ["nonexistent-id"])
    assert len(rules) == 0
