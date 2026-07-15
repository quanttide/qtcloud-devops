# TODO — `release/*` → `source/` 迁移 + `detect` 恢复单模块

> 将 `release/` 内通用工具函数提升到 `source/`，之后 `detect/` 坍缩回单文件 `detect.rs`。

## 范围

| 来源 → 目标 | 函数 |
|-------------|------|
| → `source/git.rs` | `git` / `git_check` / `is_git_repo` / `rev_list_count` / `is_working_tree_dirty` / `ref_exists`（来自 `release/util/git.rs`） |
| → `source/git.rs` | `get_changed_paths_since_last_tag`（来自 `detect/scope_util.rs`） |
| → `source/git.rs` | `parse_commit_messages`（来自 `detect/inference.rs`） |
| → `source/gh.rs` | `check_gh_installed` / `create_release` / `view_release_body` / `delete_release`（来自 `release/util/gh.rs`） |
| → `source/tag.rs` | `create_tag` / `push_tag` / `rollback_tag` / `delete_local_tag` / `delete_remote_tag` / `tag_push_refspec`（来自 `release/util/tag.rs`） |
| → `source/tag.rs` | `get_latest_tag_for_scope` / `collect_tags_with_scope` / `parse_tag` / `parse_version` / `build_version` / `apply_scope_prefix` + `VersionParts`（来自 `detect/tag_util.rs`） |

## 步骤

### 1. 创建目标文件

- [ ] **`source/git.rs`**：合并 `release/util/git.rs`（6 函数）+ `get_changed_paths_since_last_tag` + `parse_commit_messages`
- [ ] **`source/gh.rs`**：搬入 `release/util/gh.rs` 全部内容
- [ ] **`source/tag.rs`**：合并 `release/util/tag.rs` + `detect/tag_util.rs` 全部内容

### 2. 更新 `source/mod.rs`

- [ ] 添加 `pub mod git;` / `pub mod gh;` / `pub mod tag;`

### 3. 坍缩 `detect/` 回单文件

- [ ] 删除 `detect/tag_util.rs`（全部已搬至 `source/tag.rs`）
- [ ] 删除 `detect/scope_util.rs`（`get_changed_paths` 搬至 `source/git.rs`；`detect_single_scope` + `detect_project_type` 合并入 `detect/mod.rs`）
- [ ] `detect/inference.rs` 合并入 `detect/mod.rs`（移除了 `parse_commit_messages`）
- [ ] `detect/mod.rs` → 提升为 `detect.rs`，删除 `detect/` 目录
- [ ] 更新 `release/mod.rs`：`mod detect;` 不变（自动引用 `detect.rs`）

### 4. 清理 `release/util/`

- [ ] 删除 `release/util/` 整个目录
- [ ] `release/mod.rs`：移除 `pub(crate) mod util;`
- [ ] re-export 路径改为 `crate::source::{git,gh,tag}::xxx`

### 5. 更新内部引用

| 文件 | 当前 | 改为 |
|------|------|------|
| `release/audit.rs` | `util::gh::check_gh_installed` | `crate::source::gh::check_gh_installed` |
| `release/audit.rs` | `util::gh::view_release_body` | `crate::source::gh::view_release_body` |
| `release/audit.rs` | `util::git::is_working_tree_dirty` | `crate::source::git::is_working_tree_dirty` |
| `release/audit.rs` | `util::git::ref_exists` | `crate::source::git::ref_exists` |
| `release/status.rs` | `use super::util;` → `util::git::xxx` | `crate::source::git::xxx` |
| `release/detect.rs` (原 `mod.rs`) | `super::util::git::git` | `crate::source::git::git` |
| `release/detect.rs` | `tag_util::xxx` / `scope_util::xxx` / `inference::xxx` | 内部直接调用 |
| `release/detect.rs` | `use crate::source::tag::xxx` 替代原 `tag_util::xxx` | 新增 import |
| `release/publish.rs` | `super::delete_release` 等 | 不变（`release/mod.rs` re-export） |

### 6. 验证

- [ ] `cargo check` 无错误
- [ ] `cargo test --lib` 全部通过（≥335 tests）
