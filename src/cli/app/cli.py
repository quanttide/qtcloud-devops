#!/usr/bin/env python3
"""DevOps CLI — 发布管理、工作流自动化。

    >>> from app.cli import app
    >>> from typer.testing import CliRunner
    >>> runner = CliRunner()
    >>> result = runner.invoke(app, ["--help"])
    >>> result.exit_code
    0
"""

import typer

app = typer.Typer()


@app.command()
def release(
    version: str = typer.Argument(..., help="版本号（如 v0.1.0）"),
    changelog: str = typer.Option("CHANGELOG.md", "--changelog", help="CHANGELOG.md 路径"),
    repo: str = typer.Option(None, "--repo", help="GitHub 仓库（如 quanttide/quanttide-platform）"),
    dry_run: bool = typer.Option(False, "--dry-run", help="仅检查，不执行"),
    yes: bool = typer.Option(False, "--yes", "-y", help="跳过确认提示，直接发布"),
):
    """发布 Release — 预检查、打标签、推送、创建 GitHub Release"""
    from app.release import run
    from pathlib import Path
    code = run(version, Path(changelog), repo, dry_run, yes)
    raise typer.Exit(code=code)


def main():
    return app()


if __name__ == "__main__":  # pragma: no cover
    exit(main())
