from pathlib import Path
from app.release import precheck, extract_notes, confirm_release


def test_precheck_invalid_version():
    errors = precheck("invalid", Path("CHANGELOG.md"))
    assert any("版本号格式错误" in e for e in errors)


def test_precheck_changelog_not_found():
    errors = precheck("v0.1.0", Path("/nonexistent/CHANGELOG.md"))
    assert any("不存在" in e for e in errors)


def test_precheck_dirty_workspace():
    errors = precheck("v0.1.0", Path("/nonexistent/CHANGELOG.md"))
    assert any("不存在" in e for e in errors)


def test_extract_notes():
    changelog = Path("/tmp/test_changelog.md")
    changelog.write_text("""# CHANGELOG

## [0.1.0] - 2026-01-01

初始版本。

### Added

- 功能 A
- 功能 B
""")
    notes = extract_notes("v0.1.0", changelog)
    assert "功能 A" in notes
    assert "功能 B" in notes
    changelog.unlink()


def test_confirm_release_yes_flag():
    assert confirm_release("v0.1.0", "some notes", yes=True) is True


def test_confirm_release_no_input(monkeypatch):
    monkeypatch.setattr("builtins.input", lambda _: "n")
    assert confirm_release("v0.1.0", "some notes") is False


def test_confirm_release_yes_input(monkeypatch):
    monkeypatch.setattr("builtins.input", lambda _: "y")
    assert confirm_release("v0.1.0", "some notes") is True
