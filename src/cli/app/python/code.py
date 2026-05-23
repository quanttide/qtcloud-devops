"""Python wrapper for qtcloud-devops Rust native code module.

Maps to the `qtcloud_devops.code` native module compiled by maturin.
"""

from __future__ import annotations

from typing import Any

try:
    import qtcloud_devops_cli as _native
except ImportError:
    _native = None


def status(path: str = ".") -> dict[str, Any]:
    if _native is None:
        return {
            "error": "Rust native module not available (install with `pip install -e packages/code`)"
        }
    try:
        result = _native.scan_repo(path)
        return result
    except Exception as e:
        return {"error": str(e)}


def sync(name: str | None, path: str = ".") -> dict[str, Any]:
    if _native is None:
        return {
            "error": "Rust native module not available (install with `pip install -e packages/code`)"
        }
    try:
        if name:
            _native.sync_single(name, path)
        else:
            _native.sync_all(path)
        return {"status": "ok"}
    except Exception as e:
        return {"error": str(e)}


def retire(name: str, path: str = ".") -> dict[str, Any]:
    if _native is None:
        return {
            "error": "Rust native module not available (install with `pip install -e packages/code`)"
        }
    try:
        _native.retire_submodule(name, path)
        return {"status": "ok"}
    except Exception as e:
        return {"error": str(e)}
