import re
import subprocess
from pathlib import Path
from typing import Optional


def get_remote_repo() -> Optional[str]:
    """从 git remote 解析 owner/name。"""
    result = subprocess.run(
        ["git", "remote", "get-url", "origin"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    url = result.stdout.strip()
    m = re.search(r"github\.com[/:]([^/]+/[^/]+?)(?:\.git)?$", url)
    if m:
        return m.group(1)
    return None


def precheck(version: str, changelog: Path, release_only: bool = False) -> list[str]:
    errors = []

    if not re.match(
        r"^(v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?|[a-zA-Z0-9_.-]+/v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?)$",
        version,
    ):
        errors.append(f"版本号格式错误: {version}")

    if changelog.exists():
        content = changelog.read_text(encoding="utf-8")
        ver = version.split("/v", 1)[1] if "/v" in version else version.lstrip("v")
        if f"## [{ver}]" not in content:
            errors.append(f"CHANGELOG.md 未找到 {ver} 版本记录")
    else:
        errors.append(f"CHANGELOG.md 不存在: {changelog}")

    if release_only:
        result = subprocess.run(
            ["git", "tag", "-l"],
            capture_output=True,
            text=True,
        )
        existing_tags = result.stdout.strip().split("\n")
        if version not in existing_tags:
            errors.append(f"标签不存在: {version}（--release-only 需要标签已存在）")

    result = subprocess.run(
        ["git", "status", "--porcelain"],
        capture_output=True,
        text=True,
    )
    if result.stdout.strip():
        errors.append("工作区有未提交的变更")

    result = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"],
        capture_output=True,
        text=True,
    )
    branch = result.stdout.strip()
    if not branch.startswith(("main", "master", "release/")):
        errors.append(
            f"不在可发布分支上 (当前: {branch})，请切换到 main/master/release/*"
        )

    return errors


def extract_notes(version: str, changelog: Path) -> Optional[str]:
    ver = version.split("/v", 1)[1] if "/v" in version else version.lstrip("v")
    content = changelog.read_text(encoding="utf-8")
    lines = content.split("\n")
    capture = False
    notes: list[str] = []
    for line in lines:
        if line.startswith(f"## [{ver}]"):
            capture = True
            continue
        if capture:
            if line.startswith("## ["):
                break
            notes.append(line)
    text = "\n".join(notes).strip()
    return text if text else None


def confirm_release(version: str, notes: Optional[str], yes: bool = False) -> bool:
    """🧠 AI 介入点：展示检查摘要并等待用户确认"""
    print(f"\n发布版本: {version}")
    print()
    print("检查结果:")
    print("  ✓ 预检查全部通过")
    print()
    print("Release Notes 预览:")
    print(notes or "(空)")
    print()

    if yes:
        return True

    try:
        response = input("确认发布? (y/N): ").strip().lower()
        return response in ("y", "yes")
    except (EOFError, KeyboardInterrupt):
        return False


def create_tag(version: str) -> bool:
    result = subprocess.run(
        ["git", "tag", version],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"创建标签失败: {result.stderr.strip()}")
        return False
    return True


def push_tag(version: str) -> bool:
    result = subprocess.run(
        ["git", "push", "origin", version],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"推送标签失败: {result.stderr.strip()}")
        return False
    return True


def create_release(version: str, notes: str, repo: str) -> bool:
    result = subprocess.run(
        [
            "gh",
            "release",
            "create",
            version,
            "--title",
            version,
            "--notes",
            notes,
            "--repo",
            repo,
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"创建 Release 失败: {result.stderr.strip()}")
        return False
    return True


def verify_release(version: str, repo: str) -> bool:
    """🤖 规则：发布后验证"""
    result = subprocess.run(
        ["gh", "release", "view", version, "--repo", repo],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"⚠ 验证 Release 失败: {result.stderr.strip()}")
        return False
    print(result.stdout.strip())
    return True


def rollback_tag(version: str) -> None:
    """🧠 AI 介入点：标签已推送但后续失败时回滚"""
    subprocess.run(["git", "tag", "-d", version], capture_output=True)
    subprocess.run(
        ["git", "push", "origin", "--delete", version],
        capture_output=True,
        text=True,
    )
    print(f"↻ 标签 {version} 已回滚")


def run(
    version: str,
    changelog: Optional[Path] = None,
    dry_run: bool = False,
    tag_only: bool = False,
    release_only: bool = False,
    yes: bool = False,
):
    changelog = changelog or Path.cwd() / "CHANGELOG.md"

    if tag_only and release_only:
        print("错误: --tag-only 和 --release-only 不能同时使用")
        return 1

    # --- 预检查 ---
    errors = precheck(version, changelog, release_only=release_only)
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

    # --- 发布前确认 ---
    if not confirm_release(version, notes, yes=yes):
        print("已取消发布")
        return 0

    # --- 执行发布 ---
    tag_created = False

    if not release_only:
        result = subprocess.run(
            ["git", "tag", "-l"],
            capture_output=True, text=True,
        )
        tag_exists = version in result.stdout.strip().split("\n")
        if tag_exists:
            print(f"→ 标签 {version} 已存在，跳过 tag 创建")
        else:
            if not create_tag(version):
                return 1
            if not push_tag(version):
                rollback_tag(version)
                return 1
            tag_created = True
            print(f"✓ 标签 {version} 已创建并推送")

    if not tag_only:
        repo = get_remote_repo()
        if not repo:
            print("错误: 无法从 git remote 解析 GitHub 仓库")
            if tag_created:
                rollback_tag(version)
            return 1
        if not create_release(version, notes or "", repo):
            if tag_created:
                rollback_tag(version)
            return 1
        print(f"✓ GitHub Release {version} 已创建")
        print(f"  https://github.com/{repo}/releases/tag/{version}")

    return 0
