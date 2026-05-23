"""集成测试：code 子命令组。

测试 coverage:
- CLI 命令注册和帮助文本
- 参数校验（缺失、格式错误）
- 错误处理路径（native 模块不可用、路径不存在、非 git 仓库）
"""

import subprocess
from pathlib import Path

import pytest
from typer.testing import CliRunner

from qtcloud_devops_cli.cli import app

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


class TestCodeDeepIntegration:
    """深度集成测试：用真实 git 仓库验证功能正确性。"""

    def _commit_in_submodule(self, repo_path: Path, sub_name: str):
        """在子模块内创建一次新提交。"""
        sm_path = repo_path / sub_name
        (sm_path / "new-file").write_text("new content")
        subprocess.run(["git", "add", "."], cwd=sm_path, capture_output=True, check=True)
        subprocess.run(
            ["git", "commit", "-m", "new commit in submodule"],
            cwd=sm_path,
            capture_output=True,
            check=True,
        )

    def _output_contains(self, output: str, keyword: str) -> bool:
        return keyword.lower() in output.lower()

    def test_status_shows_clean_after_init(self, git_repo_with_submodule):
        """初始化后 status 显示子模块为 Clean。"""
        result = runner.invoke(app, ["code", "status", str(git_repo_with_submodule)])
        output = _all_output(result)
        assert result.exit_code == 0, output
        assert "Clean" in output or "干净" in output

    def test_status_shows_dirty_after_submodule_commit(
        self, git_repo_with_submodule
    ):
        """在子模块创建新提交后 status 显示 Dirty（git2 认为 HEAD 偏离 = 脏）。"""
        self._commit_in_submodule(git_repo_with_submodule, "libs/sub")
        result = runner.invoke(app, ["code", "status", str(git_repo_with_submodule)])
        output = _all_output(result)
        assert result.exit_code == 0, output
        assert "'Dirty'" in output or "'ahead_count': 1" in output

    def test_sync_resets_ahead_count_to_zero(self, git_repo_with_submodule):
        """sync 后 parent_pointer 追上 local_head，ahead_count 归零。"""
        self._commit_in_submodule(git_repo_with_submodule, "libs/sub")
        # 先确认 ahead_count > 0
        before = runner.invoke(app, ["code", "status", str(git_repo_with_submodule)])
        assert "'ahead_count': 1" in _all_output(before)
        # sync
        sync_result = runner.invoke(
            app, ["code", "sync", "libs/sub", "--repo", str(git_repo_with_submodule)]
        )
        assert sync_result.exit_code == 0, _all_output(sync_result)
        # 再次 status：parent_pointer == local_head（ahead_count 归零）
        after = runner.invoke(app, ["code", "status", str(git_repo_with_submodule)])
        after_output = _all_output(after)
        assert after.exit_code == 0, after_output
        assert "'ahead_count': 0" in after_output

    def test_retire_removes_submodule_from_status(self, git_repo_with_submodule):
        """retire 后 status 中子模块 url 为空（表示已去初始化）。"""
        result = runner.invoke(
            app,
            ["code", "retire", "libs/sub", "--repo", str(git_repo_with_submodule)],
        )
        output = _all_output(result)
        assert result.exit_code == 0, output
        # retire 后子模块的 url 应为空（deinit 后 url 不可用）
        status_result = runner.invoke(
            app, ["code", "status", str(git_repo_with_submodule)]
        )
        status_output = _all_output(status_result)
        assert "'url': ''" in status_output

    def test_retire_removes_gitmodules_entry(self, git_repo_with_submodule):
        """retire 后 .gitmodules 中对应条目被移除。"""
        runner.invoke(
            app,
            ["code", "retire", "libs/sub", "--repo", str(git_repo_with_submodule)],
        )
        gitmodules = git_repo_with_submodule / ".gitmodules"
        if gitmodules.exists():
            content = gitmodules.read_text()
            assert "libs/sub" not in content

    def test_code_command_help_no_traceback(self):
        """code --help 输出不包含 traceback。"""
        result = runner.invoke(app, ["code", "--help"])
        output = _all_output(result)
        assert result.exit_code == 0, output
        assert "Traceback" not in output
