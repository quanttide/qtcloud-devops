from typer.testing import CliRunner
from app.cli import app

runner = CliRunner()


def test_help():
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    assert "release" in result.stdout


def test_contract_check_invalid_path():
    result = runner.invoke(app, ["contract-check", "--contract", "/nonexistent/contract.yaml"])
    assert result.exit_code != 0


def test_contract_check_valid(contract_path):
    result = runner.invoke(app, ["contract-check", "--contract", str(contract_path)])
    assert result.exit_code == 0
    assert "约束:" in result.stdout
    assert "边界:" in result.stdout
    assert "检查:" in result.stdout
