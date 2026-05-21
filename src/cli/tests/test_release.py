from pathlib import Path
from app.release import precheck, extract_notes


def test_precheck_invalid_version():
    errors = precheck("invalid", Path("CHANGELOG.md"))
    assert any("版本号格式错误" in e for e in errors)


def test_precheck_changelog_not_found():
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
