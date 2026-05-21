import re
import subprocess
from pathlib import Path
from typing import Optional


def precheck(version: str, changelog: Path) -> list[str]:
    errors = []

    if not re.match(r"^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$", version):
        errors.append(f"版本号格式错误: {version}")

    if changelog.exists():
        content = changelog.read_text(encoding="utf-8")
        ver = version.lstrip("v")
        if f"## [{ver}]" not in content:
            errors.append(f"CHANGELOG.md 未找到 {ver} 版本记录")
    else:
        errors.append(f"CHANGELOG.md 不存在: {changelog}")

    result = subprocess.run(
        ["git", "tag", "-l"],
        capture_output=True, text=True,
    )
    existing_tags = result.stdout.strip().split("\n")
    if version in existing_tags:
        errors.append(f"标签已存在: {version}")

    result = subprocess.run(
        ["git", "status", "--porcelain"],
        capture_output=True, text=True,
    )
    if result.stdout.strip():
        errors.append("工作区有未提交的变更")

    result = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"],
        capture_output=True, text=True,
    )
    branch = result.stdout.strip()
    if not branch.startswith(("main", "master", "release/")):
        errors.append(f"不在可发布分支上 (当前: {branch})，请切换到 main/master/release/*")

    return errors


def extract_notes(version: str, changelog: Path) -> Optional[str]:
    ver = version.lstrip("v")
    content = changelog.read_text(encoding="utf-8")
    lines = content.split("\n")
    capturing = False
    notes = []
    for line in lines:
        if line.startswith(f"## [{ver}]"):
            capturing = True
            continue
        if capturing:
            if line.startswith("## ["):
                break
            notes.append(line)
    text = "\n".join(notes).strip()
    return text if text else None


def create_tag(version: str) -> bool:
    result = subprocess.run(
        ["git", "tag", version],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"创建标签失败: {result.stderr.strip()}")
        return False
    return True


def push_tag(version: str) -> bool:
    result = subprocess.run(
        ["git", "push", "origin", version],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"推送标签失败: {result.stderr.strip()}")
        return False
    return True


def create_release(version: str, notes: str, repo: str) -> bool:
    result = subprocess.run(
        [
            "gh", "release", "create", version,
            "--title", version,
            "--notes", notes,
            "--repo", repo,
        ],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"创建 Release 失败: {result.stderr.strip()}")
        return False
    return True


def run(
    version: str,
    changelog: Optional[Path] = None,
    repo: Optional[str] = None,
    dry_run: bool = False,
):
    changelog = changelog or Path.cwd() / "CHANGELOG.md"

    errors = precheck(version, changelog)
    if errors:
        print("预检查失败:")
        for err in errors:
            print(f"  ✗ {err}")
        return 1

    notes = extract_notes(version, changelog)
    print(f"\n=== Release Notes 预览 ===")
    print(notes or "(空)")
    print("=========================\n")

    if dry_run:
        print("✓ 预检查通过 (dry-run 模式，不执行)")
        return 0

    if not create_tag(version):
        return 1

    if not push_tag(version):
        return 1

    if repo:
        if not create_release(version, notes or "", repo):
            return 1

    print(f"✓ Release {version} 创建成功")
    return 0
