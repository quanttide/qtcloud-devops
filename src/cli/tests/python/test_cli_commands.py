from pathlib import Path
from unittest.mock import MagicMock

from typer.testing import CliRunner

from python.cli import app

runner = CliRunner()


def test_help():
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    assert "release" in result.stdout


def test_release_help():
    result = runner.invoke(app, ["release", "--help"])
    assert result.exit_code == 0
    assert "--version" in result.stdout


def test_release_invalid_version():
    result = runner.invoke(app, ["release", "--version", "invalid"])
    assert result.exit_code != 0


def test_release_dry_run(monkeypatch):
    changelog = Path("/tmp/test_release_dry_run_CHANGELOG.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

    def mock_run(cmd, **kw):
        if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"]:
            return MagicMock(returncode=0, stdout="main\n")
        return MagicMock(returncode=0, stdout="")

    monkeypatch.setattr("python.release.subprocess.run", mock_run)
    result = runner.invoke(
        app,
        [
            "release",
            "--version",
            "v0.1.0",
            "--changelog",
            str(changelog),
            "--dry-run",
        ],
    )
    assert result.exit_code == 0
    changelog.unlink()


def test_main_help(monkeypatch):
    from python.cli import main

    monkeypatch.setattr("sys.argv", ["python.cli", "--help"])
    try:
        main()
    except SystemExit as e:
        assert e.code == 0


def test_main_module_entry():
    import os
    import subprocess
    import sys

    root = Path(__file__).resolve().parent.parent.parent
    env = {**os.environ, "PYTHONPATH": str(root / "app")}
    result = subprocess.run(
        [sys.executable, "-m", "python.cli", "--help"],
        capture_output=True,
        text=True,
        cwd=str(root),
        env=env,
    )
    assert result.returncode == 0
    assert "release" in result.stdout
