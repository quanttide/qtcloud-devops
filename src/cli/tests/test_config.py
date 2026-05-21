from pathlib import Path


def test_settings_empty_contract_path():
    from app.config import Settings
    s = Settings(contract_path="   ")
    assert isinstance(s.contract_path, Path)
    assert str(s.contract_path).endswith("contract.yaml")


def test_settings_default_contract_path():
    from app.config import Settings
    s = Settings()
    assert str(s.contract_path).endswith("contract.yaml")


def test_settings_custom_contract_path():
    from app.config import Settings
    s = Settings(contract_path="/custom/path/contract.yaml")
    assert str(s.contract_path) == "/custom/path/contract.yaml"


def test_settings_env_prefix(monkeypatch):
    monkeypatch.setenv("QTCLOUD_DEVOPS_CONTRACT_PATH", "/env/contract.yaml")
    from app.config import Settings
    s = Settings()
    assert str(s.contract_path).endswith("contract.yaml")
