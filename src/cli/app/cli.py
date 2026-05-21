#!/usr/bin/env python3
"""qtcloud-devops-cli — DevOps 发布管理命令行工具。

基于 devops-release 技能，提供预检查、发布前确认、执行发布、验证、回滚的全流程自动化。

用法:
    qtcloud-devops release --version v0.1.0 --repo quanttide/quanttide-founder
    qtcloud-devops release --version v0.1.0 --dry-run
    qtcloud-devops release --version v0.1.0 -y
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
    repo: str = typer.Option(
        None, "--repo", help="GitHub 仓库（如 quanttide/quanttide-platform）"
    ),
    dry_run: bool = typer.Option(False, "--dry-run", help="仅检查，不执行"),
    yes: bool = typer.Option(False, "--yes", "-y", help="跳过确认提示，直接发布"),
):
    """发布 Release — 预检查、打标签、推送、创建 GitHub Release"""
    from pathlib import Path

    from app.release import run

    code = run(version, Path(changelog), repo, dry_run, yes)
    raise typer.Exit(code=code)


def main():
    return app()


if __name__ == "__main__":  # pragma: no cover
    exit(main())
