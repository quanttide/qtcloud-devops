"""集成测试：release 命令。

测试 coverage:
- CLI 命令注册和帮助文本
- 参数校验（缺失、互斥、格式错误）
- 错误处理路径（dry-run、预检查失败、tag/release 失败）
"""

from pathlib import Path
from unittest.mock import MagicMock

import pytest
from typer.testing import CliRunner

from python.cli import app

runner = CliRunner()


def _all_output(result) -> str:
    out = result.stdout or ""
    err = result.stderr or ""
    return out + err


def _mock_subprocess(monkeypatch, commands=None):
    """Mock subprocess.run with specific command results."""
    if commands is None:
        commands = {}
    default = MagicMock(returncode=0, stdout="")

    def mock_run(cmd, **kw):
        key = tuple(cmd)
        if key in commands:
            return commands[key]
        return default

    monkeypatch.setattr("python.release.subprocess.run", mock_run)


class TestReleaseCommandStructure:
    """命令注册与帮助文本"""

    def test_release_in_main_help(self):
        result = runner.invoke(app, ["--help"])
        assert result.exit_code == 0
        assert "release" in result.stdout

    def test_release_help(self):
        result = runner.invoke(app, ["release", "--help"])
        assert result.exit_code == 0
        assert "--version" in result.stdout
        assert "--tag-only" in result.stdout
        assert "--release-only" in result.stdout
        assert "--dry-run" in result.stdout

    def test_release_requires_version(self):
        result = runner.invoke(app, ["release"])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_release_version_short_flag(self):
        """-V 短 flag 应正常识别"""
        result = runner.invoke(app, ["release", "-V", "v0.1.0", "--dry-run", "-y"])
        output = _all_output(result)
        assert "Traceback" not in output


class TestReleaseDryRun:
    """dry-run 模式"""

    def test_dry_run_with_changelog(self, tmp_path, monkeypatch):
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")
        _mock_subprocess(monkeypatch, {
            ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
        })
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "--dry-run", "-y",
        ])
        output = _all_output(result)
        assert result.exit_code == 0, output
        assert "Traceback" not in output

    def test_dry_run_invalid_version(self):
        result = runner.invoke(app, [
            "release", "--version", "invalid",
            "--dry-run", "-y",
        ])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_dry_run_missing_changelog(self):
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", "/__nonexistent__/CHANGELOG.md",
            "--dry-run", "-y",
        ])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output


class TestReleaseArgumentValidation:
    """参数校验"""

    def test_tag_only_and_release_only_mutually_exclusive(self, tmp_path):
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "--tag-only", "--release-only",
            "--dry-run", "-y",
        ])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "不能同时使用" in output


class TestReleaseWithMockedSubprocess:
    """通过 mock subprocess.run 测试完整流程"""

    def test_default_flow(self, monkeypatch, tmp_path):
        """默认行为：创建标签 + 推送 + GitHub Release"""
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

        _mock_subprocess(monkeypatch, {
            ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
            ("git", "remote", "get-url", "origin"): MagicMock(returncode=0, stdout="git@github.com:quanttide/repo.git\n"),
        })
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "-y",
        ])
        output = _all_output(result)
        assert result.exit_code == 0, output
        assert "Traceback" not in output

    def test_tag_only(self, monkeypatch, tmp_path):
        """--tag-only 仅打标签不发 release"""
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

        _mock_subprocess(monkeypatch, {
            ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
        })
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "--tag-only", "-y",
        ])
        output = _all_output(result)
        assert result.exit_code == 0, output
        assert "Traceback" not in output

    def test_tag_create_failure(self, monkeypatch, tmp_path):
        """创建标签失败"""
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

        _mock_subprocess(monkeypatch, {
            ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
            ("git", "tag", "v0.1.0"): MagicMock(returncode=1, stderr="error"),
        })
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "-y",
        ])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_push_failure_triggers_rollback(self, monkeypatch, tmp_path):
        """推送标签失败应触发回滚"""
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

        calls = []

        def recorder(cmd, **kw):
            calls.append(cmd)
            if cmd == ["git", "tag", "-l"]:
                return MagicMock(returncode=0, stdout="")
            if cmd == ["git", "push", "origin", "v0.1.0"]:
                return MagicMock(returncode=1, stderr="push failed")
            if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"]:
                return MagicMock(returncode=0, stdout="main\n")
            return MagicMock(returncode=0, stdout="")

        monkeypatch.setattr("python.release.subprocess.run", recorder)
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "-y",
        ])
        output = _all_output(result)
        assert result.exit_code != 0
        assert ["git", "tag", "-d", "v0.1.0"] in calls
        assert ["git", "push", "origin", "--delete", "v0.1.0"] in calls
        assert "Traceback" not in output

    def test_release_only(self, monkeypatch, tmp_path):
        """--release-only 仅创建 GitHub Release"""
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

        _mock_subprocess(monkeypatch, {
            ("git", "tag", "-l"): MagicMock(returncode=0, stdout="v0.1.0\n"),
            ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
            ("git", "remote", "get-url", "origin"): MagicMock(returncode=0, stdout="git@github.com:quanttide/repo.git\n"),
        })
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "--release-only", "-y",
        ])
        output = _all_output(result)
        assert result.exit_code == 0, output
        assert "Traceback" not in output

    def test_release_only_tag_not_exist(self, monkeypatch, tmp_path):
        """--release-only 但标签不存在应失败"""
        changelog = tmp_path / "CHANGELOG.md"
        changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

        _mock_subprocess(monkeypatch, {
            ("git", "tag", "-l"): MagicMock(returncode=0, stdout="v0.2.0\n"),
            ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
        })
        result = runner.invoke(app, [
            "release", "--version", "v0.1.0",
            "--changelog", str(changelog),
            "--release-only", "-y",
        ])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output
