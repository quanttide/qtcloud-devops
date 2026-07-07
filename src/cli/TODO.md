# TODO — v0.10.0 plan 命令统一

> 由 `ROADMAP.md` 的 [0.10.0] 分解而来。

## plan clean 扩展

- [x] `src/plan.rs` 从 `clean_roadmap` 中抽公共函数 `clean_done_items`（仅 `- [x]` 删除逻辑）；`src/main.rs` `run_plan_clean` 改为遍历 `["ROADMAP.md", "TODO.md"]`，循环调用 `clean_done_items`
- [x] `src/main.rs` git commit message 动态拼接实际清理的文件名，而非硬编码 "roadmap items"

## plan audit 结构检查

- [x] `src/plan.rs` `plan_audit`：新增路径存在性检查，扫描 TODO 条目中的 `src/` `packages/` 等路径模式，与文件系统对照
- [x] `src/plan.rs` `plan_audit`：新增粒度检查，TODO 条目不含路径时标记 ⚠
- [x] `src/plan.rs` `edit_llm`：为不含路径的 TODO 条目自动补充文件路径（与 audit 联动消除 ⚠）
- [x] `src/plan.rs` `plan_audit`：新增孤儿 ROADMAP 条目检查
- [x] `src/plan.rs` `edit_llm`：`format_spec`（第 593 行）和 system message（第 611 行）两处都更新，支持 ROADMAP.md + TODO.md 两种格式

## 测试

- [x] `tests/cli.rs` `test_cli_plan_clean_both_files`（编排逻辑集成测试）
- [x] `src/plan.rs` tests：`test_audit_path_missing`
- [x] `src/plan.rs` tests：`test_audit_granularity_warn`

## code review 技术债清理

> 由 `qtcloud-code review src/` 静态分析生成，按 MUST → SHOULD → MAY 优先级处理。
> 当前仅记录，不修改代码。

### 🔴 MUST（长函数拆分）

- [ ] `src/main.rs:212` `main` 函数 190 行 → 按子命令拆为独立 handler
- [ ] `src/release/publish.rs:18` `publish` 函数 165 行 → 拆分校验/构建/GitHub 发布三步
- [ ] `src/plan.rs:757` `plan_audit` 126 行 → 路径检查/粒度检查/孤儿条目检查各抽函数
- [ ] `src/git.rs:299` `scan_single_submodule` 117 行 → 拆分遍历/状态计算/日志
- [ ] `src/source.rs:13` `status_to` 102 行 → 数据提取与格式化输出分离
- [ ] `src/release/audit.rs:56` `audit` 95 行 → 拆分校验规则与报告生成
- [ ] `src/code.rs:92` `audit` 91 行 → 拆分 Git 扫描与标记计数
- [ ] `src/release/detect.rs:194` `llm_decide` 87 行 → 拆分为决策/解析/回退
- [ ] `src/test.rs:125` `scan_scope` 86 行 → 拆分 scope 发现与文件遍历
- [ ] `src/plan.rs:500` `apply_rule_fixes` 113 行 → 拆分编辑与一致性检查

### 🟡 SHOULD（长函数 + 长参数列表）

- [ ] `src/git.rs:220` `scan_with_options` 76 行 → 拆分 option 解析与扫描执行
- [ ] `src/plan.rs:392` `clean_roadmap` 79 行 → 拆分格式处理与条目过滤
- [ ] `src/plan.rs:137` `parse_roadmap_str` 69 行 → 拆分解析与验证
- [ ] `src/plan.rs:617` `edit_llm` 62 行 → 拆分 prompt 组装与结果处理
- [ ] `src/plan.rs:683` `llm_audit_consistency` 55 行
- [ ] `src/build.rs:355` `audit` 54 行
- [ ] `src/release/detect.rs:284` `fallback_heuristic` 54 行
- [ ] `src/git.rs:418` `determine_submodule_status` 9 参数 → 改为 struct 传参
- [ ] `src/release/detect.rs:149` `build_version_from_decision` 7 参数 → 改为 struct 传参
- [ ] `src/release/detect.rs:341` `build_version` 7 参数 → 改为 struct 传参
- [ ] `src/build.rs:94` `build_scope_str` 6 参数 → 改为 struct 传参
- [ ] `src/code.rs:185` `count_markers` 6 参数 → 改为 struct 传参

### 🔴 MUST（缺失测试）

- [ ] `src/release/audit.rs` 缺少对应测试文件
- [ ] `src/contract.rs` 缺少对应测试文件

## 手工审计发现

> 根据 `plan audit` 规则手动检查 ROADMAP + TODO 发现以下规划自身缺陷。

### 工具缺陷（先修）

- [ ] `src/plan.rs` `extract_line_paths` 不支持行号后缀 `:N`，导致 TODO 中 `src/foo.rs:123` 类路径的文件存在性检查误报
- [ ] `src/plan.rs` `CATEGORIES` 缺少 `### Refactor`，导致 `plan edit` 将重构条目归一为 `### Changed`，复用分类

### 规划格式修复

- [ ] `ROADMAP.md:8` 条目 `code 命令重新设计：简化 status/sync 接口` 缺少反引号路径引用（孤儿条目）
- [ ] `ROADMAP.md:22` 条目 `release 后主仓库子模组指针未自动更新` 缺少反引号路径引用（孤儿条目）
