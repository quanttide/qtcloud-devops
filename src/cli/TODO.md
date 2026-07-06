# TODO — 代码清理（5 个反模式）

## 1. `Result<String, String>` → 自定义错误类型

- [x] `release/changelog.rs`: `collect_git_log`、`llm_changelog`、`write_changelog`、`ensure_changelog` — 定义 `ChangelogError` 枚举区分 I/O 错误、LLM 错误、空提交
- [x] `release/detect.rs`: `git_output`、`detect_version` — 定义 `DetectError`
- [x] `code/sync.rs`: `sync`、`sync_all` — 去掉 `format!("同步失败: {}")` 包裹
- [x] `code/status.rs`: `status` — 同上
- [x] `plan.rs`: `parse_roadmap`、`print_status_to`、`clean_roadmap`、`doctor_roadmap` — 定义 `PlanError`

## 2. 字符串版本排序 → semver

- [x] `release/changelog.rs:45`: `tags.sort_by(|a, b| b.cmp(a))` → semver 排序（`parse_tag_semver`）
- [x] `release/status.rs:162`: `get_latest_tags_by_scope` 同理
- [x] 测试确认 `v10.0.0` > `v9.0.0`

## 3. 函数职责拆分

- [x] `release/changelog.rs` `ensure_changelog`: 剥离 `git add` + `git commit`（第 5 步），改为调用方提交
- [x] `release/detect.rs` `detect_version`（~110 行）：拆分 I/O + 逻辑 + 打印职责（提取 `parse_commit_messages`、`build_version_from_decision`、`apply_scope_prefix`）
- [ ] `release/status.rs` `status_to`（~160 行）+ `contract.rs` `status_to`（~90 行）+ `build.rs` `status_to`：三个重复的状态显示函数合并或委托给库（低优先级，每个仅 3 行样板）

## 4. `pub` 收窄

- [x] `release/changelog.rs`: `collect_git_log`、`write_changelog` → private（仅 `ensure_changelog` 是真实入口）
- [x] `release/detect.rs`: `load_contract_scopes` 与 `contract::load` 重复，已删除（改用 `contract::load_scopes`）
- [x] `code/sync.rs` + `code/status.rs`: 确认 `pub` 是否必要 — `sync`/`sync_all`/`status` 均为主入口，需保留 `pub`

## 5. 错误信息统一为中文

- [x] `release/changelog.rs`: 确认已统一中文
- [x] `release/detect.rs`: 确认已统一中文
- [x] `code/*.rs`: 确认已统一中文
- [x] `git/editor.rs`: 确认已统一中文
- [x] `plan.rs`: 确认已统一中文
- [x] `main.rs`: 确认已统一中文
