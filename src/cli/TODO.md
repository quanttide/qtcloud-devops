# TODO — v0.10.0 plan 命令统一

> 由 `ROADMAP.md` 的 [0.10.0] 分解而来。

## plan clean 扩展

- [ ] `src/plan.rs` 从 `clean_roadmap` 中抽公共函数 `clean_done_items`（仅 `- [x]` 删除逻辑）；`src/main.rs` `run_plan_clean` 改为遍历 `["ROADMAP.md", "TODO.md"]`，循环调用 `clean_done_items`
- [ ] `src/main.rs` git commit message 动态拼接实际清理的文件名，而非硬编码 "roadmap items"

## plan audit 结构检查

- [ ] `src/plan.rs` `plan_audit`：新增路径存在性检查，扫描 TODO 条目中的 `src/` `packages/` 等路径模式，与文件系统对照
- [ ] `src/plan.rs` `plan_audit`：新增粒度检查，TODO 条目不含路径时标记 ⚠
- [ ] `src/plan.rs` `edit_llm`：为不含路径的 TODO 条目自动补充文件路径（与 audit 联动消除 ⚠）
- [ ] `src/plan.rs` `plan_audit`：新增孤儿 ROADMAP 条目检查
- [ ] `src/plan.rs` `edit_llm`：`format_spec`（第 593 行）和 system message（第 611 行）两处都更新，支持 ROADMAP.md + TODO.md 两种格式

## 测试

- [ ] `src/main.rs` tests：`test_clean_both_files`（编排逻辑集成测试）
- [ ] `src/plan.rs` tests：`test_audit_path_missing`
- [ ] `src/plan.rs` tests：`test_audit_granularity_warn`
