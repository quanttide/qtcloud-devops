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

## 阶段二：消除审计与发布的预检重复

- [ ] 抽取 `release/precheck.rs`
- [ ] 定义公共函数 `run_precheck(version, repo_path) → PrecheckResult`
- [ ] 从 `audit::audit()` 和 `publish::publish()` 提取公共预检链：
  ```
  validate_version → normalize_version → resolve_scope_dir → precheck_version_changelog
  ```
- [ ] 确保 `audit::audit()` 和 `publish::publish()` 调用同一份 `run_precheck`
- [ ] 为 `PrecheckResult` 写单元测试覆盖正常/异常路径

## 阶段三：拆分 `detect.rs`（~1058 行 → ~200 行/文件）

- [ ] 将 `detect.rs` 转换为 `detect/` 子目录
- [ ] 创建 `detect/mod.rs`：仅 re-export + 保留 `detect_version()` 入口
- [ ] 创建 `detect/tag_util.rs`：`parse_tag` / `parse_version` / `collect_tags_with_scope`
- [ ] 创建 `detect/inference.rs`：`llm_decide` / `call_llm_decision` / `fallback_heuristic` / `build_version_prompt` / `build_decision_from_flags`
- [ ] 创建 `detect/scope_util.rs`：`detect_project_type` / `detect_single_scope` / `get_changed_paths_since_last_tag`
- [ ] 保留私有结构体在各自文件中（`VersionParts`、`LlmDecision`、`DetectResult`、`DetectError`）
- [ ] 更新所有引用路径（`super::detect::xxx` → `super::detect::tag_util::xxx` 等）

## 阶段四：消除 `is_git_repo` 重复

- [ ] `status.rs` 的私有 `is_git_repo` 改为调用 `crate::git::is_git_repo`
- [ ] 若 `crate::git` 无此函数，从 `quanttide-devops::source::git_repo::is_git_repo` 导入
- [ ] 移除 `status.rs` 中的私有副本
- [ ] 跑测试验证行为不变
