# TODO — plan 命令升级

> 由 `ROADMAP.md` 的 [0.11.0] 版本分解而来。

## plan clean 扩展

- [ ] `src/main.rs` `run_plan_clean`：改为遍历 `["ROADMAP.md", "TODO.md"]`，复用 `clean_roadmap`
- [ ] `src/main.rs` git commit message 包含两个文件名

## plan audit 结构检查

- [ ] `src/plan.rs` `plan_audit`：新增路径存在性检查，扫描 TODO 条目中的 `src/` `packages/` 等路径模式，与文件系统对照
- [ ] `src/plan.rs` `plan_audit`：新增粒度检查，TODO 条目不含路径时标记 ⚠
- [ ] `src/plan.rs` `plan_audit`：新增孤儿 ROADMAP 条目检查
- [ ] `src/plan.rs` `edit_llm` prompt：更新为 "ROADMAP.md 和 TODO.md 格式修复助手"

## 测试

- [ ] `src/plan.rs` tests：`test_clean_both_files`
- [ ] `src/plan.rs` tests：`test_audit_path_missing`
- [ ] `src/plan.rs` tests：`test_audit_granularity_warn`
