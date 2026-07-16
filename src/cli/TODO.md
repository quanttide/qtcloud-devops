## TODO

### 架构债 — 数据模型下沉

依赖方向：`source / platform → contract → plan / test / release / build / code`

#### P0：数据模型与 I/O 解耦

- [x] `CiRun` → `quanttide-devops::stage::build`
- [x] `TestSummary` / `Coverage` / `AuditReport` / `is_io_fn` → `quanttide-devops::stage::test`
- [ ] `source/git/submodule` 的数据模型与 I/O 分离：`SubmoduleStatus` / `Submodule` / `RepoState` / `AggregateStatus` / `HealthIssue` 中纯数据结构定义移至 toolkit 的 `source/git/submodule`，I/O 方法（`scan()` 等）变为 A 中自由函数或 toolkit trait 实现
- [ ] `build::ScopeInfo` — 依赖 contract 引用，评估是否可变为自有类型后下沉
- [ ] `code::SyncStatus` / `ComponentStatus` / `StatusReport` — code 命令的展示模型，评估是否可下沉

#### P1：pub 可见性审计

- [ ] 排查所有 `pub(crate)` 定义，确认是否应改为 `pub`（尤其是 `source/git/*`、`test/*`、`build/*` 中的数据模型）
- [ ] `doctor.rs` 中的 `detect_used_languages()` 与 B 的 `config_file::detect_languages` 重叠，整合后改为 `pub`

#### P2：stage 模块对齐

- [ ] `src/build/` — 纯编排函数（status/clean/audit）保留，领域逻辑（`check_command` / `check_manifest_file` / `check_dependencies` / `resolve_workflow`）下沉到 toolkit `stage/build.rs`
- [ ] `src/test/` — 纯编排函数保留，解析器（`parse_lcov_coverage` / `parse_cobertura_coverage`）下沉到 toolkit `source/coverage.rs`
- [ ] `src/release/` — `get_latest_tags_by_scope()` / `resolve_scope_dir()` / `precheck_version_changelog()` / `extract_notes()` 下沉到 toolkit `stage/release.rs`
- [ ] `src/plan/` — 纯格式化/解析逻辑（`clean_done_items` / `doctor_file`）下沉到 toolkit `source/roadmap.rs`
- [ ] `src/doctor.rs` — 系统诊断检测逻辑，评估下沉到 toolkit `source/diagnostics.rs`

### 架构债 — 代码质量

#### MUST

- [x] `src/source/changelog.rs` → `crate::release::normalize_version`：反向依赖
- [x] `src/source/mod.rs` → `crate::contract`：`detect_used_languages()` 和配套函数提取到独立 diagnostics 模块
- [x] `src/release/mod.rs` 内联 contract 薄包装：移除了 `validate_version()` 和 `normalize_version()` 两个纯委托函数
- [ ] `src/release/mod.rs`：`precheck_version_changelog()`、`extract_notes()`、`confirm_release()`、`parse_github_repo()` —— 这些是 release 业务逻辑，但在依赖分层上属于 contract-consumer 层，标记待审

#### SHOULD

- [ ] `src/release/detect.rs`（707 行）：scope 版本探测逻辑和 release 决策逻辑深度耦合。应考虑按"版本探测"和"发布决策"拆分。
- [ ] `src/code/audit.rs`（818 行）：代码审计逻辑过长，应按审计规则拆分。
- [ ] `tests/mock.rs`（716 行）：测试基础设施过长，应按领域拆分。
- [ ] `src/release/status.rs`（529 行）：状态展示和 release 数据收集耦合在一起。
- [ ] `src/test/tests.rs`（722 行）：测试用例过长，应按被测试模块拆分。
- [ ] `src/main.rs`（527 行）：CLI 入口过长，子命令逻辑应继续拆分。

#### MAY

- [ ] 43/45 文件缺少 `//!` 模块文档（覆盖率 4%）

### 版本与发布纪律

- [ ] 建立发布流程：CLI 发布前先发布 toolkit（如有 toolkit 变更），确保 B 版本≥A 依赖的最小版本
- [ ] 每次在 CLI 中新增 domain 概念类型时，先在 toolkit 中定义

### 已完成（历史）

- [x] `source/git.rs` 按概念拆分为 `source/git/` 子模块（mod/repo/status/log/diff）
- [x] `source/git_tag.rs` → `source/git/tag.rs`，`source/git_submodule.rs` → `source/git/submodule.rs`
- [x] `diagnostics.rs` → `doctor.rs`，`source` 子命令 → `doctor`
- [x] toolkit 的 `git_repo.rs`/`git_tag.rs` 集中到 `source/git/` 文件夹
- [x] `CiRun` / `TestSummary` / `Coverage` / `AuditReport` / `is_io_fn` 下沉到 toolkit
