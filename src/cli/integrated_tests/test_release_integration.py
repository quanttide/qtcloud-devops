"""集成测试：release 命令。

测试 coverage:
- CLI 命令注册和帮助文本
- 参数校验（缺失、互斥、格式错误）
- 错误处理路径（dry-run、预检查失败、tag/release 失败）
"""

from pathlib import Path
from unittest.mock import MagicMock

from typer.testing import CliRunner

from python.cli import app

runner = CliRunner()


def _all_output(result) -> str:
    out = result.stdout or ""
    err = result.stderr or ""
    return out + err


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
        monkeypatch.setattr(
            "python.release.subprocess.run",
            lambda cmd, **kw: MagicMock(
                returncode=0,
                stdout="main\n" if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"] else "",
            ),
        )
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



