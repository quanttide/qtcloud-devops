## TODO

### 架构债

依赖方向：`source / platform → contract → plan / test / release / build / code`

#### MUST

- [x] `src/source/changelog.rs` → `crate::release::normalize_version`：反向依赖
- [x] `src/source/mod.rs` → `crate::contract`：`detect_used_languages()` 和配套函数提取到独立 `diagnostics` 模块
- [x] `src/release/mod.rs` 内联 contract 薄包装：移除了 `validate_version()` 和 `normalize_version()` 两个纯委托函数
- [ ] `src/release/mod.rs`：`precheck_version_changelog()`、`extract_notes()`、`confirm_release()`、`parse_github_repo()` ——
      这些是 release 业务逻辑，但在依赖分层上属于 contract-consumer 层，标记待审

#### SHOULD

- [ ] `src/release/detect.rs`（707 行）：scope 版本探测逻辑和 release 决策逻辑深度耦合。
      应考虑按"版本探测"和"发布决策"拆分。
- [ ] `src/code/audit.rs`（818 行）：代码审计逻辑过长，应按审计规则拆分。
- [ ] `tests/mock.rs`（716 行）：测试基础设施过长，应按领域拆分。
- [ ] `src/release/status.rs`（529 行）：状态展示和 release 数据收集耦合在一起。
- [ ] `src/test/tests.rs`（722 行）：测试用例过长，应按被测试模块拆分。
- [ ] `src/main.rs`（527 行）：CLI 入口过长，子命令逻辑应继续拆分。

#### MAY

- [ ] 43/45 文件缺少 `//!` 模块文档（覆盖率 4%）
