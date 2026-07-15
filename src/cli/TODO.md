# TODO — release 模块重构

> 源自 `docs/release/refactoring-plan.md`，按阶段渐进推进。

## ~~阶段一：提取 `util.rs`（低风险，纯搬移）~~ ✅

- [x] 创建 `release/util/mod.rs` — re-export 全部子模块
- [x] 创建 `release/util/tag.rs`，从 `mod.rs` 搬入：`create_tag`、`delete_local_tag`、`delete_remote_tag`、`rollback_tag`、`tag_push_refspec`、`push_tag`
- [x] 创建 `release/util/gh.rs`，从各处搬入/提取：`create_release`、`delete_release`、`view_release_body`、`check_gh_installed`
- [x] 创建 `release/util/git.rs`，收敛分散的 git 命令调用：统一 `git()` + `git_check()` + `rev_list_count()` + `is_working_tree_dirty()` + `ref_exists()` + `is_git_repo()`
- [x] 更新 `release/mod.rs`：保留业务函数 + re-export util
- [x] 更新 `release/audit.rs`：改用 `util::gh::check_gh_installed` + `util::gh::view_release_body` + `util::git::is_working_tree_dirty` + `util::git::ref_exists`
- [x] 更新 `release/status.rs`：改用 `util::git::is_git_repo` + `util::git::rev_list_count`，移除私有 `is_git_repo` 副本
- [x] 更新 `release/detect.rs`：`git_output` 委托给 `util::git::git`

## ~~阶段二：消除审计与发布的预检重复~~ ✅

> **注意：** `publish()` 在 changelog 生成后才检查 changelog，与 `audit()` 顺序不同，因此 publish 仍直接调用 `normalize_version` + `resolve_scope_dir`，不通过 `run_precheck`。

- [x] 抽取 `release/precheck.rs`
- [x] 定义公共函数 `run_precheck(version, repo_path) → PrecheckResult`
- [x] 从 `audit::audit()` 提取公共预检链 → 改用 `run_precheck`
  ```
  validate_version → normalize_version → resolve_scope_dir → precheck_version_changelog
  ```
- [x] `publish::publish()` 因 changelog 检查时机不同，仍直接调用原函数
- [x] 为 `PrecheckResult` 写单元测试覆盖正常/异常路径（5 个测试）

## ~~阶段三：拆分 `detect.rs`（~1058 行 → ~200 行/文件）~~ ✅

> **注意：** `get_latest_tag_for_scope` 仍通过 `pub(crate) use` 从 `detect/mod.rs` 暴露，外部引用路径不变（`super::detect::get_latest_tag_for_scope`）。

- [x] 将 `detect.rs` 转换为 `detect/` 子目录
- [x] 创建 `detect/mod.rs`：`detect_version()` 入口 + `DetectError` + `DetectResult` + `git_output`
- [x] 创建 `detect/tag_util.rs`：`parse_tag` / `parse_version` / `collect_tags_with_scope` / `get_latest_tag_for_scope` / `build_version` / `apply_scope_prefix` / `VersionParts`
- [x] 创建 `detect/inference.rs`：`llm_decide` / `call_llm_decision` / `fallback_heuristic` / `build_version_prompt` / `build_decision_from_flags` / `parse_commit_messages` / `build_version_from_decision` / `LlmDecision`
- [x] 创建 `detect/scope_util.rs`：`detect_project_type` / `detect_single_scope` / `get_changed_paths_since_last_tag`
- [x] 保留私有结构体在各自文件内
- [x] 所有测试同步迁移到对应子模块（29 + 15 + 10 = 54 个测试全部通过）

## ~~阶段四：消除 `is_git_repo` 重复~~ ✅

> `status.rs` 中的私有 `is_git_repo` 已在之前被移除（或从未存在）。测试中的裸 `is_git_repo()` 引用已在阶段二修复为 `util::git::is_git_repo()`。生产代码（`collect_all` / `count_unreleased_in_dir`）已在使用 `util::git::is_git_repo`。

- [x] `status.rs` 中无私有 `fn is_git_repo`
- [x] 所有引用均指向 `util::git::is_git_repo`
- [x] 测试通过验证行为不变
