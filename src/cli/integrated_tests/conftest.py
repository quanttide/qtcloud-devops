"""Shared test fixtures."""

import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest


@pytest.fixture
def git_repo_with_submodule(tmp_path: Path) -> Path:
    """创建一个包含子模块的临时 git 仓库。

    用于需要真实 git 仓库的集成测试。
    跳过条件：系统没有 `git` 命令。
    """
    if not shutil.which("git"):
        pytest.skip("git not available")

    parent = tmp_path / "parent"
    sub = tmp_path / "sub"

    # 创建子仓库
    sub.mkdir()
    subprocess.run(["git", "init"], cwd=sub, capture_output=True, check=True)
    subprocess.run(
        ["git", "config", "user.email", "test@test.com"], cwd=sub, capture_output=True
    )
    subprocess.run(["git", "config", "user.name", "Test"], cwd=sub, capture_output=True)
    (sub / "README.md").write_text("# sub")
    subprocess.run(["git", "add", "."], cwd=sub, capture_output=True, check=True)
    subprocess.run(
        ["git", "commit", "-m", "init"], cwd=sub, capture_output=True, check=True
    )

    # 创建父仓库并添加子模块
    parent.mkdir()
    subprocess.run(["git", "init"], cwd=parent, capture_output=True, check=True)
    subprocess.run(
        ["git", "config", "user.email", "test@test.com"],
        cwd=parent,
        capture_output=True,
    )
    subprocess.run(
        ["git", "config", "user.name", "Test"], cwd=parent, capture_output=True
    )
    (parent / "README.md").write_text("# parent")
    subprocess.run(["git", "add", "."], cwd=parent, capture_output=True, check=True)
    subprocess.run(
        ["git", "commit", "-m", "init"], cwd=parent, capture_output=True, check=True
    )
    subprocess.run(
        ["git", "submodule", "add", str(sub), "libs/sub"],
        cwd=parent,
        capture_output=True,
        check=True,
    )
    subprocess.run(
        ["git", "commit", "-m", "add submodule"],
        cwd=parent,
        capture_output=True,
        check=True,
    )

    return parent
