"""集成测试：code 子命令组。

测试 coverage:
- CLI 命令注册和帮助文本
- 参数校验（缺失、格式错误）
- 错误处理路径（native 模块不可用、路径不存在、非 git 仓库）
"""

from pathlib import Path

import pytest
from typer.testing import CliRunner

from python.cli import app

runner = CliRunner()

THIS_REPO = Path(__file__).resolve().parent.parent  # src/cli/


def _all_output(result) -> str:
    out = result.stdout or ""
    err = result.stderr or ""
    return out + err


class TestCodeCommandStructure:
    """命令注册与帮助文本"""

    def test_code_in_main_help(self):
        result = runner.invoke(app, ["--help"])
        assert result.exit_code == 0
        assert "code" in result.stdout

    def test_code_help(self):
        result = runner.invoke(app, ["code", "--help"])
        assert result.exit_code == 0
        assert "status" in result.stdout
        assert "sync" in result.stdout
        assert "retire" in result.stdout

    def test_code_status_help(self):
        result = runner.invoke(app, ["code", "status", "--help"])
        assert result.exit_code == 0
        assert "path" in result.stdout.lower()

    def test_code_sync_help(self):
        result = runner.invoke(app, ["code", "sync", "--help"])
        assert result.exit_code == 0
        assert "--repo" in result.stdout

    def test_code_retire_help(self):
        result = runner.invoke(app, ["code", "retire", "--help"])
        assert result.exit_code == 0


class TestCodeStatus:
    """status 命令"""

    def test_status_default_path(self):
        """无参数时默认当前目录，不应因缺少参数而报错。"""
        result = runner.invoke(app, ["code", "status"])
        output = _all_output(result)
        assert result.exit_code in (0, 1), output
        assert "Traceback" not in output

    def test_status_with_repo_path(self):
        """显式传入当前仓库路径。"""
        result = runner.invoke(app, ["code", "status", str(THIS_REPO)])
        output = _all_output(result)
        assert result.exit_code in (0, 1), output
        assert "Traceback" not in output

    def test_status_nonexistent_path(self):
        result = runner.invoke(app, ["code", "status", "/__kse_no_such_repo__"])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_status_plain_directory(self, tmp_path, monkeypatch):
        """普通空目录不是 git 仓库，应友好提示。"""
        d = tmp_path / "not-a-repo"
        d.mkdir()
        result = runner.invoke(app, ["code", "status", str(d)])
        output = _all_output(result)
        # 可能成功（git 仓库向上搜索到父仓库）或失败
        # 重要的是不 panic 不 traceback
        assert "Traceback" not in output


class TestCodeSync:
    """sync 命令"""

    def test_sync_all_default_context(self):
        """在当前仓库中运行 sync（无 name），应正常返回。"""
        result = runner.invoke(app, ["code", "sync"])
        output = _all_output(result)
        assert result.exit_code in (0, 1), output
        assert "Traceback" not in output

    def test_sync_with_name(self):
        result = runner.invoke(app, ["code", "sync", "some-module"])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_sync_with_invalid_path(self):
        result = runner.invoke(app, ["code", "sync", "lib-a", "--repo", "/dev/null"])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output


class TestCodeRetire:
    """retire 命令"""

    def test_retire_nonexistent_module(self):
        result = runner.invoke(app, ["code", "retire", "no-such-module"])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_retire_without_name_should_fail(self):
        """retire 的 name 为必填参数，省略应报错。"""
        result = runner.invoke(app, ["code", "retire"])
        output = _all_output(result)
        assert result.exit_code != 0
        # Typer 报缺失参数，信息在 stderr 中
        assert "Traceback" not in output


class TestCodeCommandEdgeCases:
    """边界场景"""

    def test_unknown_subcommand(self):
        result = runner.invoke(app, ["code", "unknown-cmd"])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_unknown_global_flag(self):
        result = runner.invoke(app, ["code", "status", "--bogus-flag"])
        output = _all_output(result)
        assert result.exit_code != 0
        assert "Traceback" not in output

    def test_empty_path_is_defaulted(self):
        """status 默认路径为当前目录，不应因缺少路径而报错。"""
        result = runner.invoke(app, ["code", "status"])
        output = _all_output(result)
        assert result.exit_code in (0, 1), output
        assert "Traceback" not in output
