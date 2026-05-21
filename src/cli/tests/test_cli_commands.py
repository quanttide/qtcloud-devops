from pathlib import Path
from unittest.mock import MagicMock
from typer.testing import CliRunner
from app.cli import app

runner = CliRunner()


def test_help():
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    assert "release" in result.stdout


def test_release_invalid_version():
    result = runner.invoke(app, ["release", "invalid"])
    assert result.exit_code != 0


def test_release_dry_run(tmp_path, monkeypatch):
    changelog = tmp_path / "CHANGELOG.md"
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

    def mock_run(cmd, **kw):
        if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"]:
            return MagicMock(returncode=0, stdout="main\n")
        return MagicMock(returncode=0, stdout="")
    monkeypatch.setattr("app.release.subprocess.run", mock_run)
    result = runner.invoke(app, ["release", "v0.1.0", "--changelog", str(changelog), "--dry-run"])
    assert result.exit_code == 0


def test_contract_check_invalid_path():
    result = runner.invoke(app, ["contract-check", "--contract", "/nonexistent/contract.yaml"])
    assert result.exit_code != 0


def test_contract_check_valid(contract_path):
    result = runner.invoke(app, ["contract-check", "--contract", str(contract_path)])
    assert result.exit_code == 0
    assert "约束:" in result.stdout
    assert "边界:" in result.stdout
    assert "检查:" in result.stdout


def test_main_help(monkeypatch):
    from app.cli import main
    monkeypatch.setattr("sys.argv", ["app.cli", "--help"])
    try:
        main()
    except SystemExit as e:
        assert e.code == 0


def test_main_module_entry():
    import subprocess
    import sys
    root = Path(__file__).resolve().parent.parent
    result = subprocess.run(
        [sys.executable, "-m", "app.cli", "--help"],
        capture_output=True, text=True,
        cwd=str(root),
    )
    assert result.returncode == 0
    assert "release" in result.stdout
