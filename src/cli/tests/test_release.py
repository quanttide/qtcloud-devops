from pathlib import Path
from unittest.mock import MagicMock

from app.release import (
    precheck, extract_notes, confirm_release,
    create_tag, push_tag, create_release, verify_release,
    rollback_tag, run, get_remote_repo,
)


def mock_subprocess(commands, default=None):
    if default is None:
        default = MagicMock(returncode=0, stdout="")
    def _mock(cmd, **kw):
        key = tuple(cmd)
        if key in commands:
            return commands[key]
        return default
    return _mock


def git_precheck_ok():
    return mock_subprocess({
        ("git", "tag", "-l"): MagicMock(stdout=""),
        ("git", "status", "--porcelain"): MagicMock(stdout=""),
        ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(stdout="main\n"),
    })


def test_precheck_invalid_version():
    errors = precheck("invalid", Path("CHANGELOG.md"))
    assert any("版本号格式错误" in e for e in errors)


def test_precheck_changelog_not_found():
    errors = precheck("v0.1.0", Path("/nonexistent/CHANGELOG.md"))
    assert any("不存在" in e for e in errors)


def test_precheck_tag_exists(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", mock_subprocess({
        ("git", "tag", "-l"): MagicMock(stdout="v0.1.0\nv0.2.0\n"),
        ("git", "status", "--porcelain"): MagicMock(stdout=""),
        ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(stdout="main\n"),
    }))
    errors = precheck("v0.1.0", Path("/tmp/CHANGELOG.md"))
    assert any("标签已存在" in e for e in errors)


def test_precheck_dirty_workspace(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", mock_subprocess({
        ("git", "tag", "-l"): MagicMock(stdout=""),
        ("git", "status", "--porcelain"): MagicMock(stdout=" M CHANGELOG.md\n"),
        ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(stdout="main\n"),
    }))
    errors = precheck("v0.2.0", Path("/tmp/CHANGELOG.md"))
    assert any("工作区有未提交的变更" in e for e in errors)


def test_precheck_wrong_branch(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", mock_subprocess({
        ("git", "tag", "-l"): MagicMock(stdout=""),
        ("git", "status", "--porcelain"): MagicMock(stdout=""),
        ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(stdout="feature/foo\n"),
    }))
    errors = precheck("v0.2.0", Path("/tmp/CHANGELOG.md"))
    assert any("不在可发布分支上" in e for e in errors)


def test_precheck_no_changelog_content(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", git_precheck_ok())
    changelog = Path("/tmp/test_precheck_no_content.md")
    changelog.write_text("# CHANGELOG\n\n## [0.2.0]\n\n内容\n")
    errors = precheck("v9.9.9", changelog)
    assert any("未找到" in e for e in errors)
    changelog.unlink()


def test_precheck_all_pass(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", git_precheck_ok())
    changelog = Path("/tmp/test_precheck_pass.md")
    changelog.write_text("# CHANGELOG\n\n## [0.2.0]\n\n内容\n")
    errors = precheck("v0.2.0", changelog)
    assert len(errors) == 0
    changelog.unlink()


def test_extract_notes():
    changelog = Path("/tmp/test_changelog.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0] - 2026-01-01\n\n初始版本。\n\n### Added\n\n- 功能 A\n- 功能 B\n")
    notes = extract_notes("v0.1.0", changelog)
    assert "功能 A" in notes
    assert "功能 B" in notes
    changelog.unlink()


def test_extract_notes_multiple_sections():
    changelog = Path("/tmp/test_changelog_multi.md")
    changelog.write_text("# CHANGELOG\n\n## [0.2.0] - 2026-02-01\n\n版本 0.2.0 内容。\n\n## [0.1.0] - 2026-01-01\n\n初始版本。\n\n### Added\n\n- 功能 A\n")
    notes = extract_notes("v0.2.0", changelog)
    assert "版本 0.2.0" in notes
    assert "功能 A" not in notes
    changelog.unlink()


def test_extract_notes_not_found():
    changelog = Path("/tmp/test_changelog_empty.md")
    changelog.write_text("# CHANGELOG\n\n## [0.2.0]\n\n其他版本\n")
    notes = extract_notes("v9.9.9", changelog)
    assert notes is None
    changelog.unlink()


def test_confirm_release_yes_flag():
    assert confirm_release("v0.1.0", "some notes", yes=True) is True


def test_confirm_release_no_input(monkeypatch):
    monkeypatch.setattr("builtins.input", lambda _: "n")
    assert confirm_release("v0.1.0", "some notes") is False


def test_confirm_release_yes_input(monkeypatch):
    monkeypatch.setattr("builtins.input", lambda _: "y")
    assert confirm_release("v0.1.0", "some notes") is True


def test_confirm_release_eof(monkeypatch):
    monkeypatch.setattr("builtins.input", lambda _: (_ for _ in ()).throw(EOFError()))
    assert confirm_release("v0.1.0", "some notes") is False


def test_confirm_release_keyboard_interrupt(monkeypatch):
    monkeypatch.setattr("builtins.input", lambda _: (_ for _ in ()).throw(KeyboardInterrupt()))
    assert confirm_release("v0.1.0", "some notes") is False


def test_create_tag_success(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=0))
    assert create_tag("v0.1.0") is True


def test_create_tag_failure(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=1, stderr="错误"))
    assert create_tag("v0.1.0") is False


def test_push_tag_success(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=0))
    assert push_tag("v0.1.0") is True


def test_push_tag_failure(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=1, stderr="错误"))
    assert push_tag("v0.1.0") is False


def test_create_release_success(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=0))
    assert create_release("v0.1.0", "notes", "quanttide/repo") is True


def test_create_release_failure(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=1, stderr="错误"))
    assert create_release("v0.1.0", "notes", "quanttide/repo") is False


def test_verify_release_success(monkeypatch):
    mock = MagicMock(returncode=0, stdout="Release v0.1.0\nURL: ...")
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: mock)
    assert verify_release("v0.1.0", "quanttide/repo") is True


def test_verify_release_failure(monkeypatch):
    mock = MagicMock(returncode=1, stderr="not found")
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: mock)
    assert verify_release("v0.1.0", "quanttide/repo") is False


def test_rollback_tag(monkeypatch):
    calls = []
    monkeypatch.setattr(
        "app.release.subprocess.run",
        lambda cmd, **kw: calls.append(cmd) or MagicMock(stdout=""),
    )
    rollback_tag("v0.1.0")
    assert calls[0] == ["git", "tag", "-d", "v0.1.0"]
    assert calls[1] == ["git", "push", "origin", "--delete", "v0.1.0"]


def test_get_remote_repo_ssh(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(
        returncode=0, stdout="git@github.com:quanttide/quanttide-platform.git\n"
    ))
    assert get_remote_repo() == "quanttide/quanttide-platform"


def test_get_remote_repo_https(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(
        returncode=0, stdout="https://github.com/quanttide/quanttide-platform\n"
    ))
    assert get_remote_repo() == "quanttide/quanttide-platform"


def test_get_remote_repo_failure(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=1, stderr=""))
    assert get_remote_repo() is None


def test_run_dry_run(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", git_precheck_ok())
    changelog = Path("/tmp/test_run_dry.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")
    assert run("v0.1.0", changelog, dry_run=True) == 0
    changelog.unlink()


def test_run_dry_run_with_errors():
    assert run("invalid", Path("/nonexistent"), dry_run=True) == 1


def test_run_cancelled(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", git_precheck_ok())
    monkeypatch.setattr("builtins.input", lambda _: "n")
    changelog = Path("/tmp/test_run_cancel.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")
    assert run("v0.1.0", changelog) == 0
    changelog.unlink()


def test_run_create_tag_failure(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", mock_subprocess({
        ("git", "tag", "v0.1.0"): MagicMock(returncode=1, stderr="错误"),
        ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
    }, default=MagicMock(returncode=0, stdout="")))
    changelog = Path("/tmp/test_run_tag_fail.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")
    assert run("v0.1.0", changelog, yes=True) == 1
    changelog.unlink()


def test_run_push_tag_failure_triggers_rollback(monkeypatch):
    calls = []
    def recorder(cmd, **kw):
        calls.append(cmd)
        if cmd == ["git", "push", "origin", "v0.1.0"]:
            return MagicMock(returncode=1, stderr="错误")
        if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"]:
            return MagicMock(returncode=0, stdout="main\n")
        return MagicMock(returncode=0, stdout="")

    monkeypatch.setattr("app.release.subprocess.run", recorder)
    changelog = Path("/tmp/test_run_push_fail.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")
    assert run("v0.1.0", changelog, yes=True) == 1
    assert ["git", "tag", "-d", "v0.1.0"] in calls
    assert ["git", "push", "origin", "--delete", "v0.1.0"] in calls
    changelog.unlink()


def test_run_default_creates_tag_only(monkeypatch):
    """默认行为：只打标签，不发 GitHub Release"""
    changelog = Path("/tmp/test_run_default.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

    calls = []
    def recorder(cmd, **kw):
        calls.append(cmd)
        if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"]:
            return MagicMock(returncode=0, stdout="main\n")
        return MagicMock(returncode=0, stdout="")

    monkeypatch.setattr("app.release.subprocess.run", recorder)
    assert run("v0.1.0", changelog, yes=True) == 0
    assert ["git", "tag", "v0.1.0"] in calls
    assert not any("create" in c for c in calls)
    changelog.unlink()


def test_run_release_only(monkeypatch):
    """--release-only 只创建 GitHub Release，不打标签"""
    changelog = Path("/tmp/test_run_release_only.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

    calls = []
    def recorder(cmd, **kw):
        calls.append(cmd)
        if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"]:
            return MagicMock(returncode=0, stdout="main\n")
        if cmd == ["git", "remote", "get-url", "origin"]:
            return MagicMock(returncode=0, stdout="git@github.com:quanttide/repo.git\n")
        if "view" in str(cmd):
            return MagicMock(returncode=0, stdout="Release v0.1.0\nURL: ...")
        return MagicMock(returncode=0, stdout="")

    monkeypatch.setattr("app.release.subprocess.run", recorder)
    assert run("v0.1.0", changelog, release_only=True, yes=True) == 0
    assert not any(cmd == ["git", "tag", "v0.1.0"] for cmd in calls)
    assert any("create" in c for c in calls)
    changelog.unlink()


def test_run_release_only_fails_without_remote(monkeypatch):
    """--release-only 无法解析 remote 时报错"""
    changelog = Path("/tmp/test_run_rel_no_remote.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

    monkeypatch.setattr("app.release.subprocess.run", mock_subprocess({
        ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(returncode=0, stdout="main\n"),
        ("git", "remote", "get-url", "origin"): MagicMock(returncode=1, stderr=""),
    }, default=MagicMock(returncode=0, stdout="")))

    assert run("v0.1.0", changelog, release_only=True, yes=True) == 1
    changelog.unlink()


def test_run_release_only_skips_precheck_tag_check(monkeypatch):
    """--release-only 预检查不因标签已存在而失败"""
    changelog = Path("/tmp/test_run_release_only_tag_exists.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")

    calls = []
    def recorder(cmd, **kw):
        calls.append(cmd)
        if cmd == ["git", "tag", "-l"]:
            return MagicMock(returncode=0, stdout="v0.1.0\n")
        if cmd == ["git", "rev-parse", "--abbrev-ref", "HEAD"]:
            return MagicMock(returncode=0, stdout="main\n")
        if cmd == ["git", "remote", "get-url", "origin"]:
            return MagicMock(returncode=0, stdout="git@github.com:quanttide/repo.git\n")
        if "view" in str(cmd):
            return MagicMock(returncode=0, stdout="Release v0.1.0\n")
        return MagicMock(returncode=0, stdout="")

    monkeypatch.setattr("app.release.subprocess.run", recorder)
    # 标签 v0.1.0 已存在，但 release_only=True 应该通过
    assert run("v0.1.0", changelog, release_only=True, yes=True) == 0
    changelog.unlink()


def test_precheck_release_only_skips_tag_check(monkeypatch):
    """--release-only 时不检查标签是否已存在"""
    monkeypatch.setattr("app.release.subprocess.run", mock_subprocess({
        ("git", "tag", "-l"): MagicMock(stdout="v0.1.0\n"),
        ("git", "status", "--porcelain"): MagicMock(stdout=""),
        ("git", "rev-parse", "--abbrev-ref", "HEAD"): MagicMock(stdout="main\n"),
    }))
    changelog = Path("/tmp/test_precheck_release_only.md")
    changelog.write_text("# CHANGELOG\n\n## [0.1.0]\n\n内容\n")
    errors = precheck("v0.1.0", changelog, release_only=True)
    assert all("标签已存在" not in e for e in errors)
    assert len(errors) == 0
    changelog.unlink()


def test_run_uses_default_changelog(monkeypatch):
    monkeypatch.setattr("app.release.subprocess.run", lambda *a, **kw: MagicMock(returncode=0, stdout="main\n"))
    assert run("v0.1.0", dry_run=True) == 1
