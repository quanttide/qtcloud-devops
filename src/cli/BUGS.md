# BUGS

当前已知但暂不修复的问题。

## M1 — maturin sdist 构建失败（已修复）

**现象**
`uv build` 执行时 `maturin pep517 write-sdist` 返回 exit status 1，构建 sdist 失败。wheel 构建正常。

**原因**
`pyproject.toml` 位于 `packages/python/` 时，maturin 找不到 `source-dir` 对应的 `Cargo.toml`。

**修复**
将 `pyproject.toml` 移回项目根目录（`src/cli/pyproject.toml`），`python-source = "packages/python"`。maturin 从项目根目录构建时能正确解析所有路径。

---

## M2 — Windows 构建失败（libgit2-sys）

**现象**
`cargo build --release --target x86_64-pc-windows-msvc` 链接失败：

```
libgit2-sys: unresolved external symbol __imp_OpenProcessToken
```

**原因**
`libgit2-sys`（vendored-libgit2）在 Windows 上需要 `advapi32` 等系统库，默认编译配置未包含。需要额外配置 Windows SDK 链接参数。

**影响**
- CI 的 `build-binaries (windows-latest)` job 失败
- Windows 平台无预编译二进制

**替代方案**
- macOS / Linux 二进制可用
- Windows 用户可通过 WSL 使用 Linux 版本
- `cargo install` 在用户本地有合适工具链时可编译

**状态**
待修复。需在 `build.rs` 或 `Cargo.toml` 中添加 Windows 系统库链接配置。
