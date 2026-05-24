# BUGS

当前已知但暂不修复的问题。

## M1 — maturin sdist 构建失败

**现象**
`uv build` 执行时 `maturin pep517 write-sdist` 返回 exit status 1，构建 sdist 失败。wheel 构建正常。

**原因**
`pyo3` 从无条件依赖改为 optional（`dep:pyo3`）后，maturin 的 sdist 构建流程可能在某些配置下找不到 pyo3 依赖。具体触发条件待排查。

**影响**
- PyPI 发布时 wheel 正常，sdist 不可用
- CI 的 `build-package` job 失败

**替代方案**
- PyPI 发布依赖 wheel，sdist 影响较小
- 可通过 `--sdist` 参数跳过或单独构建

**状态**
待排查。影响 v0.3.0 发布流程。

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
