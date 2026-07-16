## TODO

### 代码审计

#### MUST

- [x] `src/test.rs`: 拆分 `print_scope_audit` — 圈复杂度 11
- [x] `src/plan.rs`: 拆分 `fn`（圈复杂度 15，86 行，嵌套 8 层）
- [x] `src/build.rs`: 拆分 `fn`（圈复杂度 11，78 行，嵌套 5 层）
- [x] `src/release/detect.rs`: 拆分 `fn`（88 行，大幅超限）
- [x] `src/release/audit.rs`: 拆分 `fn`（135 行，大幅超限，嵌套 5 层）
- [x] `src/release/status.rs`: 拆分 `fn`（69 行）
- [x] `src/release/publish.rs`: 拆分 `fn`（54 行）
- [x] `src/code/audit.rs`: 拆分 `scan_one`（70 行）
- [x] `src/source/git.rs`: 拆分 `scan_all`（嵌套 5 层）
- [ ] 提取 `source/tag.rs` 纯函数到 toolkit（等待 issue #6）

#### SHOULD

- [x] `src/main.rs`: 拆分 `run_release_audit`（43 行）
- [x] `src/main.rs`: 拆分 `run_plan_clean`（59 行）
- [x] `src/build.rs`: 拆分 `check_dependencies`（41 行）
- [~] 其他 SHOULD 条目：函数较长但无实质复杂度（模板/匹配/渲染），无需拆分

#### MAY

- [ ] 为 32 个文件添加 `//!` 模块级文档（当前覆盖率 6%）
- [ ] 清理 TODO/FIXME/HACK 标记（103 处，8.6‰）
- [ ] 修复遗留函数签名命名（`fn` 占位名）
