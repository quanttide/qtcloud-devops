#!/usr/bin/env python3
"""qtcloud-devops-cli — DevOps 发布管理命令行工具。

基于 devops-release 技能，提供预检查、发布前确认、执行发布、验证、回滚的全流程自动化。

用法:
    qtcloud-devops release --version v0.1.0                   # 标签 + GitHub Release
    qtcloud-devops release --version v0.1.0 --tag-only        # 仅创建 Git 标签
    qtcloud-devops release --version v0.1.0 --release-only    # 仅创建 GitHub Release
    qtcloud-devops release --version v0.1.0 --dry-run         # 仅检查
    qtcloud-devops release --version v0.1.0 -y                # 跳过确认
"""

import typer

app = typer.Typer()


@app.callback()
def main_callback() -> None: ...


@app.command()
def release(
    version: str = typer.Option(..., "--version", "-V", help="版本号（如 v0.1.0）"),
    changelog: str = typer.Option(
        "CHANGELOG.md", "--changelog", help="CHANGELOG.md 路径"
    ),
    dry_run: bool = typer.Option(False, "--dry-run", help="仅检查，不执行"),
    tag_only: bool = typer.Option(
        False, "--tag-only", help="仅创建 Git 标签（默认行为，显式声明意图）"
    ),
    release_only: bool = typer.Option(
        False, "--release-only", help="仅创建 GitHub Release，跳过 Git 标签"
    ),
    yes: bool = typer.Option(False, "--yes", "-y", help="跳过确认提示，直接发布"),
):
    """发布 Release。

    默认行为：创建 Git 标签并推送 + GitHub Release（仓库从 git remote 自动检测）。
    --tag-only：仅打标签，跳过 GitHub Release。
    --release-only：仅为已有标签创建 GitHub Release（跳过标签创建）。
    """
    from pathlib import Path

    from app.release import run

    code = run(version, Path(changelog), dry_run, tag_only, release_only, yes)
    raise typer.Exit(code=code)


def main():
    return app()


if __name__ == "__main__":  # pragma: no cover
    exit(main())
