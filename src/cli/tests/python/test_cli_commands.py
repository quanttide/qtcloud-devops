from pathlib import Path
from unittest.mock import MagicMock

from typer.testing import CliRunner

from qtcloud_devops_cli.cli import app

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

    monkeypatch.setattr("qtcloud_devops_cli.release.subprocess.run", mock_run)
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
    from qtcloud_devops_cli.cli import main

    monkeypatch.setattr("sys.argv", ["qtcloud_devops_cli.cli", "--help"])
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
        [sys.executable, "-m", "qtcloud_devops_cli.cli", "--help"],
        capture_output=True,
        text=True,
        cwd=str(root),
        env=env,
    )
    assert result.returncode == 0
    assert "release" in result.stdout


class TestCodeStatusFormatting:
    """code status 格式化输出"""

    def _mock_status(self, monkeypatch, return_value):
        monkeypatch.setattr(
            "qtcloud_devops_cli.code.status", lambda path: return_value
        )

    def test_no_submodules(self, monkeypatch):
        self._mock_status(monkeypatch, {
            "root_path": "/repo", "parent_dirty": False,
            "submodules": [], "total": 0, "clean_count": 0,
            "needs_attention": [],
        })
        result = runner.invoke(app, ["code", "status", "/repo"])
        assert "没有子模块" in result.stdout
        assert "干净的" in result.stdout

    def test_dirty_parent(self, monkeypatch):
        self._mock_status(monkeypatch, {
            "root_path": "/repo", "parent_dirty": True,
            "submodules": [], "total": 0, "clean_count": 0,
            "needs_attention": [],
        })
        result = runner.invoke(app, ["code", "status", "/repo"])
        assert "有未提交的变更" in result.stdout

    def test_clean_submodule(self, monkeypatch):
        self._mock_status(monkeypatch, {
            "root_path": "/repo", "parent_dirty": False,
            "submodules": [{"name": "libs/a", "path": "libs/a", "url": "",
                            "tracked_branch": "main",
                            "parent_pointer": "", "local_head": "",
                            "remote_head": "", "status": "Clean",
                            "ahead_count": 0, "behind_count": 0,
                            "remote_unreachable": False}],
            "total": 1, "clean_count": 1, "needs_attention": [],
        })
        result = runner.invoke(app, ["code", "status", "/repo"])
        assert "✅" in result.stdout
        assert "Clean" in result.stdout

    def test_dirty_submodule_with_ahead(self, monkeypatch):
        self._mock_status(monkeypatch, {
            "root_path": "/repo", "parent_dirty": False,
            "submodules": [{"name": "libs/a", "path": "libs/a", "url": "",
                            "tracked_branch": "main",
                            "parent_pointer": "", "local_head": "",
                            "remote_head": "", "status": "Dirty",
                            "ahead_count": 3, "behind_count": 0,
                            "remote_unreachable": False}],
            "total": 1, "clean_count": 0,
            "needs_attention": ["libs/a"],
        })
        result = runner.invoke(app, ["code", "status", "/repo"])
        assert "🔴" in result.stdout
        assert "Dirty" in result.stdout
        assert "+3" in result.stdout

    def test_all_status_icons(self, monkeypatch):
        statuses = ["Clean", "Dirty", "AheadOfParent", "BehindRemote",
                    "Detached", "Orphaned", "Uninitialized"]
        icons = ["✅", "🔴", "⬆", "⬇", "⚠", "💀", "⚪"]
        subs = [{"name": f"libs/{s}", "path": f"libs/{s}", "url": "",
                 "tracked_branch": "main",
                 "parent_pointer": "", "local_head": "", "remote_head": "",
                 "status": s, "ahead_count": 0, "behind_count": 0,
                 "remote_unreachable": False} for s in statuses]
        self._mock_status(monkeypatch, {
            "root_path": "/repo", "parent_dirty": False,
            "submodules": subs, "total": 7, "clean_count": 1,
            "needs_attention": [s["name"] for s in subs if s["status"] != "Clean"],
        })
        result = runner.invoke(app, ["code", "status", "/repo"])
        for icon in icons:
            assert icon in result.stdout, f"缺少图标 {icon}"
        assert "需关注: " in result.stdout

    def test_error_returns_exit_code_1(self, monkeypatch):
        self._mock_status(monkeypatch, {"error": "test error"})
        result = runner.invoke(app, ["code", "status", "/repo"])
        assert result.exit_code != 0
        assert "test error" in result.stderr
