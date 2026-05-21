from app.contract import load_contract, find_checks_for_action, resolve_rules


def test_load_contract(contract_path):
    contract = load_contract(contract_path)
    assert "contracts" in contract
    assert "checks" in contract
    assert "boundaries" in contract


def test_find_checks_for_action(contract_path):
    contract = load_contract(contract_path)
    checks = find_checks_for_action(contract, "git tag")
    assert len(checks) == 1
    assert checks[0]["id"] == "pre-tag-check"


def test_resolve_rules(contract_path):
    contract = load_contract(contract_path)
    rules = resolve_rules(contract, ["release-changelog-required", "release-clean-working-tree"])
    assert len(rules) == 2
    assert rules[0]["id"] == "release-changelog-required"
    assert rules[1]["id"] == "release-clean-working-tree"
