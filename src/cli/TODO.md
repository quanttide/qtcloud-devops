# TODO — 代码清理（5 个反模式）

## 1. `Result<String, String>` → 自定义错误类型

- [ ] `release/changelog.rs`: `collect_git_log`、`llm_changelog`、`write_changelog`、`ensure_changelog` — 定义 `ChangelogError` 枚举区分 I/O 错误、LLM 错误、空提交
- [ ] `release/detect.rs`: `git_output`、`detect_version` — 定义 `DetectError`
- [ ] `code/sync.rs`: `sync`、`sync_all` — 去掉 `format!("同步失败: {}")` 包裹
- [ ] `code/status.rs`: `status` — 同上
- [ ] `plan.rs`: `parse_roadmap`、`print_status_to`、`clean_roadmap`、`doctor_roadmap` — 定义 `PlanError`

## 2. 字符串版本排序 → semver

- [ ] `release/changelog.rs:45`: `tags.sort_by(|a, b| b.cmp(a))` → 复用 `detect.rs` 的 `parse_version` + semver 排序
- [ ] `release/status.rs:162`: `get_latest_tags_by_scope` 同理
- [ ] 测试确认 `v10.0.0` > `v9.0.0`

## 3. 函数职责拆分

- [ ] `release/changelog.rs` `ensure_changelog`: 剥离 `git add` + `git commit`（第 5 步），改为调用方提交
- [ ] `release/detect.rs` `detect_version`（~110 行）：拆分 I/O + 逻辑 + 打印职责
- [ ] `release/status.rs` `status_to`（~160 行）+ `contract.rs` `status_to`（~90 行）+ `build.rs` `status_to`：三个重复的状态显示函数合并或委托给库

## 4. `pub` 收窄

- [ ] `release/changelog.rs`: `collect_git_log`、`write_changelog` → `pub(crate)` 或 private（仅 `ensure_changelog` 是真实入口）
- [ ] `release/detect.rs`: `load_contract_scopes` 与 `contract::load` 重复，确认后删除
- [ ] `code/sync.rs` + `code/status.rs`: 确认 `pub` 是否必要

## 5. 错误信息统一为中文

- [ ] `release/changelog.rs`: 混杂英文错误（`LLM 调用失败` + `git log 失败`）→ 统一中文
- [ ] `release/detect.rs`: 混杂 → 统一中文
- [ ] `code/*.rs`: `format!("扫描失败: {}")`、`format!("同步失败: {}")` — 已中文，确认统一
- [ ] `git/editor.rs`: `format!("子模块 push 失败: {}")` 等 — 已中文，确认统一
- [ ] `plan.rs`: `format!("读取 {} 失败")` — 已中文，确认统一
- [ ] `main.rs`: `format!("git add 失败: {}")` 等 — 已中文，确认统一
