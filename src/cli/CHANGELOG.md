# CHANGELOG

## [0.3.0-rc.6] - 2026-05-24

修复 maturin wheel 构建（pyproject.toml 回到项目根目录），Windows 构建添加 build.rs。

## [0.3.0-rc.5] - 2026-05-24

修复 pyo3 无条件编译问题（macOS 构建失败），CI 重测。

## [0.3.0-rc.3] - 2026-05-24

重测 CI，移除 build 流程中的 `cargo test`，修复 pyo3 无条件编译问题。

## [0.3.0-rc.2] - 2026-05-24

重测 CI，移除 build 流程中的 `cargo test`。

## [0.3.0-rc.1] - 2026-05-24

测试 CI 构建与发布流程（build-cli → publish-crate / publish-pypi）。无功能变更。

完整变更日志待 v0.3.0 正式版补充。

## [0.2.3] - 2026-05-24

### Fixed

- CI: wheel 用 `--auditwheel skip` 构建，上传到 GitHub Release 作为附件
- CI: sdist 单独用 `uv build --sdist` 构建并发布到 PyPI
- 版本从 0.2.2 升到 0.2.3 绕过 PyPI 文件重名限制

## [0.2.2] - 2026-05-24

### Fixed

- CI: 改用 `PyO3/maturin-action@v1` 构建 wheel，支持 manylinux

## [0.2.1] - 2026-05-24

### Fixed

- 添加 `manylinux = "2_28"` 修复 PyPI 发布时 wheel 平台标签不被接受的问题

## [0.2.0] - 2026-05-24

Rust 原生子模块管理引擎（`code` 命令）。

### Added

- `code status` 命令：三路 commit 比对 + 7 种子模块状态分类，格式化表格输出
- `code sync` 命令：端到端子模块同步（推送子模块 → 更新父指针 → 推送父仓库）
- `code retire` 命令：自动化反注册（deinit + .gitmodules + index 清理）
- `code status` 输出 `parent_dirty` 字段，无子模块时退化到普通 git 状态检测
- Rust 64 个测试（48 单元 + 6 二进制 + 10 集成），Python 88 个测试
- 覆盖率（Python 94%, Rust 95.8%）记录到 STATUS.md
- 子模组管理文档 docs/code.md

### Changed

- Python 包名 `python` → `qtcloud_devops_cli`，native 模块改为 `._native` 子模块
- `app/` 目录重组为 `src/`，Rust 代码移至 `src/` 根目录
- `sync` 从只更新本地指针改为完整端到端同步（含 push）
- 测试目录分组：`tests/python/`、`tests/rust/`、`integrated_tests/`

### Fixed

- `code.py` sync/retire 函数改为 try/except 返回 dict（之前返回 None 导致 TypeError）
- `#[pymodule]` 函数名与 `module-name` 不匹配导致的 ImportError
- 子模块 orphaned 判定中 `merge_base` 无共同祖先时应为 orphaned

## [0.1.0] - 2026-05-22

CLI 接口重构与文档体系建立。

### Added

- `--tag-only` / `--release-only` 参数，支持分开执行 tag 和 GitHub Release
- 从 `git remote get-url origin` 自动检测 GitHub 仓库，移除 `--repo` 参数
- AGENTS.md CLI 设计规则固化
- README.md、docs/index.md、docs/commands.md、docs/low-level-api.md 文档体系

### Changed

- 默认行为：标签 + GitHub Release（之前仅标签）
- Tag 已存在时默认模式跳过 tag 创建继续发 release

### Fixed

- `--release-only` 预检查验证 tag 必须存在
- 默认模式预检查不再因 tag 已存在而拒绝

## [0.0.2] - 2026-05-21

### Fixed

- 版本号格式校验支持 scope 前缀（`cli/vX.Y.Z` / `python/vX.Y.Z`）
- CHANGELOG 版本提取逻辑修正，scope 前缀版本不再影响查询

### Added

- STATUS.md：记录工具已知盲区（依赖完整性、uv.lock 同步等）

## [0.0.1] - 2026-05-21

初始版本。

### Added

- `release` 命令：预检查、发布前确认、执行发布、验证、回滚全流程自动化
- `release --version/-V`：版本号参数
- `release --dry-run`：仅检查不执行
- `release -y`：跳过确认直接发布
