"""Python wrapper for qtcloud-devops Rust native code module.

Maps to the `qtcloud_devops.code` native module compiled by maturin.
"""

from __future__ import annotations

from __future__ import annotations

from typing import Any

try:
    import qtcloud_devops_code as _native
except ImportError:
    _native = None


def status(path: str = ".") -> dict[str, Any]:
    if _native is None:
        return {"error": "Rust native module not available (install with `pip install -e packages/code`)"}
    return _native.scan_repo(path)


def sync(name: str | None, path: str = ".") -> dict[str, Any]:
    if _native is None:
        return {"error": "Rust native module not available (install with `pip install -e packages/code`)"}
    if name:
        return _native.sync_single(name, path)
    return _native.sync_all(path)


def retire(name: str, path: str = ".") -> dict[str, Any]:
    if _native is None:
        return {"error": "Rust native module not available (install with `pip install -e packages/code`)"}
    return _native.retire_submodule(name, path)
