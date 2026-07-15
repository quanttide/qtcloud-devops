# release 模块重构计划

> 基于 `docs/release/dependency-graph.md` 的分析提炼。
> 目标：消除重复、收敛系统调用、拆分过长的 `mod.rs`。

---

## 发现的问题

### 1. `mod.rs` 职责过重

当前 `mod.rs` 同时承担：
- **re-export**——向 `pub use` 外部模块
- **工具函数**——tag/git/gh/remote 操作
- **scope 解析**——`load_scopes_map`、`get_latest_tags_by_scope`、`resolve_scope_dir`

`docs/release/publish.md` 的模块架构图已画出一个 `util.rs`，但实际代码中这些函数全放在 `mod.rs` 里。
这既是代码质量缺口，也是文档与实现的不一致。

**方案**：将 tag、git、gh 三类操作拆入 `util.rs`。

### 2. `git` 命令调用分散在 4 处

| 位置 | 调用 |
|------|------|
| `mod.rs` | `git rev-parse --git-dir`、`git remote get-url origin`、`git push` |
| `audit.rs` | `git status --porcelain`、`git rev-parse refs/tags/...` |
| `status.rs` | `git rev-list --count tag..HEAD` |
| `detect.rs` | `git tag --list`、`git rev-parse`、`git log --oneline`、`git diff --name-only`、`git rev-parse --show-toplevel` |

没有统一的 git 命令封装层，相同的 git 命令在不同地方反复拼参数。

**方案**：收敛到 `util.rs` 或独立的 `git_util.rs`。

### 3. `audit` 和 `publish` 共享相同的预检链

```
validate_version → normalize_version → resolve_scope_dir → precheck_version_changelog
```

`audit::audit()` 和 `publish::publish()` 各自执行这条链，检查逻辑重复。

**方案**：抽取 `precheck` 子模块或共享函数。

### 4. `detect.rs` 文件过长（~1058 行含测试）

职责混杂了三个维度：
- tag 解析和收集（`parse_tag`、`parse_version`、`collect_tags_with_scope`）
- LLM 版本推断（`llm_decide`、`call_llm_decision`、`fallback_heuristic`）
- scope / 项目类型检测（`detect_single_scope`、`detect_project_type`）

**方案**：按职责拆为 `tag_util.rs`、`version_inference.rs`、`scope_util.rs`。

### 5. `gh CLI` 调用未封装

| 函数 | 位置 | gh 命令 |
|------|------|---------|
| `create_release` | mod.rs | `gh release create` |
| `delete_release` | mod.rs | `gh release delete` |
| `audit_github_release` | audit.rs | `gh release view --json body` |

三种操作各自独立拼字符串，没有统一的 gh 操作封装。

**方案**：收敛到 `util.rs` 中的 gh 操作组。

### 6. `is_git_repo` 重复实现

- `status.rs` 内有私有 `is_git_repo`
- Toolkit `source/git_repo.rs` 已有同名函数

**方案**：改用 toolkit 的 `is_git_repo` 或暴露到 `crate::git` 共享。

---

## 重构计划

### 阶段一：提取 `util.rs`（低风险，纯搬移）

将 `mod.rs` 中的工具函数按类别搬入新文件 `util.rs`：

**新增 `release/util/tag.rs`**：
- `create_tag`
- `delete_local_tag`
- `delete_remote_tag`
- `rollback_tag`
- `tag_push_refspec`
- `push_tag`

**新增 `release/util/gh.rs`**：
- `create_release`
- `delete_release`
- `view_release_body`（从 `audit_github_release` 提取的 `gh release view`）
- `check_gh_installed`

**新增 `release/util/git.rs`**：
- 收敛各处 `std::process::Command::new("git")` 调用
- 提供 `git(args, cwd) → Result<String, String>` 统一入口

**新增 `release/util/mod.rs`**：
- re-export

`mod.rs` 仅保留 re-export 和 `resolve_scope_dir`、`load_scopes_map` 等 scope 解析函数。

### 阶段二：消除审计与发布的预检重复

- 抽取 `release/precheck.rs`
- 公共函数 `run_precheck(version, repo_path) → PrecheckResult`
- `audit::audit()` 和 `publish::publish()` 都调用它

### 阶段三：拆分 `detect.rs`

```
detect/
├── mod.rs          → 仅 re-export + detect_version()
├── tag_util.rs     → parse_tag / parse_version / collect_tags_with_scope
├── inference.rs    → llm_decide / call_llm_decision / fallback_heuristic
└── scope_util.rs   → detect_project_type / detect_single_scope
```

### 阶段四：消除 `is_git_repo` 重复

- `status.rs` 的 `is_git_repo` 改为调用 `crate::git::is_git_repo` 或 toolkit 版本
- 移除私有副本

---

## 优先级与风险评估

| 阶段 | 风险 | 工作量 | 收益 |
|------|------|--------|------|
| 一（util.rs） | 低——纯搬移，不改变逻辑 | 小 | 消除文档/代码不一致，mod.rs 减负 |
| 二（precheck） | 中——需确保 audit 和 publish 的语义等价 | 中 | 消除重复逻辑 |
| 三（detect 拆分） | 中——拆模块涉及路径和可见性调整 | 中 | ~300 行/文件 → ~100 行/文件 |
| 四（去重） | 低——简单替换 | 小 | 消灭重复代码 |
