## TODO

### 代码审计

#### MUST

- [x] `src/test.rs`: 拆分 `print_scope_audit` — 圈复杂度 11
- [x] `src/plan.rs`: 拆分 `fn`（圈复杂度 15，86 行，嵌套 8 层）
- [x] `src/build.rs`: 拆分 `fn`（圈复杂度 11，78 行，嵌套 5 层）
- [x] `src/release/detect.rs`: 拆分 `fn`（88 行，大幅超限）
- [x] `src/release/audit.rs`: 拆分 `fn`（135 行，大幅超限，嵌套 5 层）
- [ ] `src/release/status.rs`: 拆分 `fn`（69 行）
- [ ] `src/release/publish.rs`: 拆分 `fn`（54 行）
- [ ] `src/code/audit.rs`: 拆分 `scan_one`（70 行）
- [ ] `src/source/git.rs`: 拆分 `scan_all`（嵌套 5 层）
- [ ] 提取 `source/tag.rs` 纯函数到 toolkit（`parse_version` / `build_version` / 等待 issue #6）

#### SHOULD

- [ ] `src/main.rs`: 拆分 `run_release_audit`（43 行）
- [ ] `src/main.rs`: 拆分 `run_plan_clean`（59 行）
- [ ] `src/plan.rs`: 拆分 `try_print_plan_file`（52 行）
- [ ] `src/plan.rs`: 拆分 `print_progress`（50 行）
- [x] `src/build.rs`: 拆分 `check_dependencies`（41 行）
- [ ] `src/contract.rs`: 拆分 `fn`（70 行）
- [ ] `src/release/detect.rs`: 拆分 `build_version_prompt`（55 行）
- [ ] `src/release/detect.rs`: 拆分 `build_decision_from_flags`（52 行）
- [ ] `src/release/publish.rs`: 拆分测试函数（45 行）
- [ ] `src/release/audit.rs`: 拆分 `audit_github_release`（58 行）
- [ ] `src/code/status.rs`: 拆分 `test_status_report_counts`（41 行）
- [ ] `src/source/git.rs`: 拆分 `scan_with_options`（60 行）
- [ ] `src/source/git.rs`: 拆分 `scan_single_submodule`（56 行）
- [ ] `src/source/changelog.rs`: 拆分 `ensure_changelog`（46 行）
- [ ] `src/source/mod.rs`: 拆分 `build_language_sections`（58 行）
- [ ] 将测试 `tests/mock.rs`（716 行）按 scope 拆分
- [ ] `src/test.rs`（1472 行）按 scope 拆分

#### MAY

- [ ] 为 32 个文件添加 `//!` 模块级文档（当前覆盖率 6%）
- [ ] 清理 TODO/FIXME/HACK 标记（103 处，8.6‰）
- [ ] 修复遗留函数签名命名（`fn` 占位名）
