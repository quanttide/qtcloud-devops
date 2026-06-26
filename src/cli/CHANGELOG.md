# CHANGELOG

## [0.6.0] - 2026-06-26

### Added

- CHANGELOG 自动生成：`release publish` 内置 LLM 调用（`quanttide-agent` 库），
  CHANGELOG 缺失当前版本时自动从 git 提交记录生成
  - 环境变量 `LLM_API_KEY` 配置，未配置时降级为提示文本，不阻塞发布
  - 写入后自动 `git add + git commit`，确保标签包含 CHANGELOG 修改
  - 版本号自动 normalize（去掉 scope 和 `v` 前缀），与预检查格式一致

### Fixed

- `write_changelog` 版本号 normalize 后写入，与 `precheck_version_changelog` 搜索格式一致
- `ensure_changelog` 写入后自动提交，避免标签创建时遗漏 CHANGELOG 修改
- `pyproject.toml` 版本与 `Cargo.toml` 同步，修复 CI validate-version 失败

## [0.6.0-rc.3] - 2026-06-26

### Changed
- Bumped version to v0.6.0-rc.3.

## [0.6.0-rc.2] - 2026-06-26

### Fixed
- CHANGELOG 自动生成：版本号 normalize 后写入，与 precheck 搜索格式一致

## [0.6.0-rc.1] - 2026-06-26

### Added

- CHANGELOG 自动生成：`release publish` 内置 LLM 调用（`quanttide-agent`），
  CHANGELOG 缺失当前版本时自动从 git 提交记录生成
- `llm_changelog` 集成到发布流程，环境变量 `LLM_API_KEY` 配置
  - 未配置时降级为提示文本，不阻塞发布

## [0.5.0-rc.2] - 2026-06-10

### Fixed

- Cargo.toml / pyproject.toml 版本同步 rc 版本号，修复 CI version mismatch

## [0.5.0-rc.1] - 2026-06-10

测试 CI 构建与发布流程（build-cli → publish-crate / publish-pypi）。无功能变更。

## [0.5.1] - 2026-06-11

### Fixed

- `code sync` 改为双向同步：先 fetch + rebase 子模块远程更新，再 push 子模块、更新父指针、push 父仓库
- 父仓库 push 失败后自动回滚父仓库提交

## [0.5.0] - 2026-06-10

### Breaking

- 模块结构重构：`commands/` + `model/` → `code/` + `git/` + `release/` 三子领域
- 删除 `release stage` 命令，只保留 `release publish`
- 删除 `release retire`、`code retire`、`release status` 命令
- 删除 release 状态跟踪（`ReleaseStatus` 枚举、`FileStorage` journal 持久化）
  - `ReleaseStatus`、`ReleaseRecord`、`TransitionError` 移入 `packages/toolkit`
- `pub use git::submodule` 移除（外部引用需改为 `git::submodule::*`）
- `HealthIssue.status` 从 `SubmoduleStatus` 枚举改为 `String`
- `RepoState.parent_dirty` 字段移除

### Added

- `code::status()` 返回业务类型 `StatusReport`（`SyncStatus` 四态）
- `code::sync()` / `code::sync_all()` 封装 sync 原语
- `RepoState::scan_offline()` 支持跳过 fetch
- `SyncStatus::label()` 中文标签输出
- 测试覆盖率从 65.3% 提升至 77.85%

### Fixed

- `--offline` 标志实际生效
- `sync` 签名从 `repo.signature()` 读取，原子化三阶段，前序 fetch
- `python.rs` 绑定和语法错误修复
- `release publish` API 简化，移除 `--pre-release` 参数
- Cargo.toml / pyproject.toml 版本同步

### Changed

- `sync` 跳过 detached HEAD 和无远程场景
- `preflight.sh` 增加 `--features python` 验证

### Breaking

- 模块结构重构：`commands/` + `model/` → `code/` + `git/` + `release/` 三子领域
- 删除 `release stage` 命令，只保留 `release publish`
  - `code/`：业务层，纯抽象，不暴露 git 概念
  - `git/`：事实源底层，所有 git 操作
  - `release/`：发布子领域
- 删除 `release retire` 命令
- 删除 `code retire` 命令
- 删除 `release status` 命令
- 删除 release 状态跟踪（`ReleaseStatus` 枚举、`FileStorage` journal 持久化）
  - `ReleaseStatus`、`ReleaseRecord`、`TransitionError` 移入 `packages/toolkit`
- `pub use git::submodule` 移除（外部引用需改为 `git::submodule::*`）
- `HealthIssue.status` 从 `SubmoduleStatus` 枚举改为 `String`
- `RepoState.parent_dirty` 字段移除

### Added

- `code::status()` 返回业务类型 `StatusReport`（`SyncStatus` 四态：Synced/PendingPush/PendingPull/Conflict）
- `code::sync()` / `code::sync_all()` 封装 sync 原语
- `RepoState::scan_offline()` / `scan_with_options(offline)` 支持跳过 fetch
- `SyncStatus::label()` 中文标签输出
- 测试覆盖率从 65.3% 提升至 77.85%

### Fixed

- `--offline` 标志实际生效（跳过子模块 fetch）
- `sync` 提交作者从 `repo.signature()` 读取，替代硬编码 `"kse" <kse@local>`
- `sync` 三个阶段原子化：push_parent 失败时 revert parent commit
- `sync` 前先 fetch，确保远端状态最新
- `python.rs` `retire_submodule` 绑定移除、语法错误修复
- `release publish` API 简化：移除 `--pre-release` 参数，确认仅依赖 `-y`
- `src/cli/AGENTS.md` 同步更新 CLI 设计规则

### Changed

- `sync` 推送跳过 detached HEAD 和无远程场景
- `preflight.sh` 包含 `cargo build --features python` 验证

## [0.4.3] - 2026-05-25

### Fixed

- `release stage`/`release publish` 强制检查 CHANGELOG，缺失对应版本记录时拒绝执行

### Added

- 新增 6 个测试覆盖 CHANGELOG 拒绝场景（单元 + 集成 + CLI 子进程三层）
- `src/cli/AGENTS.md` 新建，与子模组级 AGENTS 拆分职责

## [0.4.2] - 2026-05-25

### Fixed

- `code status` fetch 子模块而非主仓库，确保 remote_head 实时

## [0.4.1] - 2026-05-25

### Fixed

- CLI 命令重组：`stage`/`publish`/`retire` 移入 `release` 子命令组
  - 旧：`qtcloud-devops stage -v ...`
  - 新：`qtcloud-devops release stage -v ...`
- `publish` 不再要求先 `stage`，正式版可直接发布
- `create_tag`、`create_release` 幂等（tag/Release 已存在时跳过）

## [0.4.1-rc.1] - 2026-05-25

## [0.4.0] - 2026-05-25

### Breaking

- `cancel` 命令移除
- `stage` 仅接受预发布版本（含 `-rc.N`、`-alpha.N` 等后缀）
- `stage` 改为推送 tag + 创建 GitHub Release

### Added

- `publish --registry <name>` 指定发布目标
- `code status --offline` 跳过 fetch
- CLI 集成测试（13 个）

### Fixed

- `code status` 先 fetch 再扫描，确保 remote_head 实时
- `sync` 输出改为单行聚合格式

### Changed

- `--dry-run` 下放到 `sync`/`retire` 子命令级别

## [0.4.0-rc.2] - 2026-05-25

### Breaking

- `cancel` 命令移除
- `stage` 仅接受预发布版本（含 `-rc.N`、`-alpha.N` 等后缀）
- `stage` 改为推送 tag（不再只是写 journal）

### Added

- `publish --registry <name>` 指定发布目标
- `code status --offline` 跳过 fetch
- CLI 集成测试（13 个）

### Fixed

- `code status` 先 fetch 再扫描，确保 remote_head 实时
- `sync` 输出改为单行聚合格式

### Changed

- `--dry-run` 下放到 `sync`/`retire` 子命令级别

## [0.3.4] - 2026-05-25

### Fixed

- `code status` Dirty 误判（ahead_count > 0 时不标 Dirty）
- `code status` 默认 fetch，确保 remote_head 实时
- `sync` 输出改为单行聚合格式

### Changed

- `--dry-run` 下放到 `code sync`/`code retire` 子命令级别
- `cancel` 标记为废弃（v0.4.0 将移除）

### Added

- `code status --offline` 跳过 fetch

## [0.3.3] - 2026-05-25（CI 失败，未发布）

## [0.3.2] - 2026-05-25

### Added

- `release status` 命令：从 journal 查询发布状态

### Changed

- `release-status` → `release status`（`Release` 子命令组）

### Fixed

- Python 构建修复：`python.rs` 适配新的 `model::code` 模块结构

## [0.3.1] - 2026-05-25（CI 失败，未发布）

## [0.3.0] - 2026-05-24

### Added

- 发布状态机：`stage` / `publish` / `cancel` / `retire` 四个命令（BREAKING）
- 事件溯源持久化：`.quanttide/devops/release-journal.jsonl` 追加写
- 多平台 CI 构建（Linux x86_64 / macOS arm64 / Windows x86_64）
- 发布 CI（build-cli → publish-crate + publish-pypi）
- `scripts/preflight.sh`、`scripts/validate-version.sh`、`scripts/validate-changelog.sh`
- 安装文档、发布教程、BUGS.md

### Changed

- 纯 Rust CLI，移除 Python 入口（BREAKING：`release --version` → `stage -v` + `publish -v`）
- PyPI 包降级为 native 库分发渠道
- `pyo3` 从无条件依赖改为 optional（`python` feature）
- Cargo.toml / pyproject.toml 构建配置分离
- `src/qtcloud_devops_cli/` → `packages/python/`
- AGENTS.md 补充发布纪律

### Fixed

- git 命令通过 `git -C <repo_path>` 执行，不再污染 CWD
- Windows 构建：添加 `build.rs` 链接 `advapi32`
- macOS 构建：`pyo3` optional 修复
- CI 版本校验：tag 版本正确比对 Cargo.toml + pyproject.toml

---

## [0.3.0-rc.8] - 2026-05-24

修复 crates.io license, PyPI dist path: --allow-dirty, maturin build direct（pyproject.toml 回到项目根目录），Windows 构建添加 build.rs。

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
