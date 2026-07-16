## TODO

### 架构债

依赖方向：`source / platform → contract → plan / test / release / build / code`

#### MUST

- [ ] `src/source/changelog.rs` → `crate::release::normalize_version`：反向依赖。
      source 层不应依赖 release 层。`normalize_version` 是 contract 定义的标准，
      应改为 `crate::contract::normalize_version`，直接调用 contract 而非 release 的薄包装。
- [ ] `src/source/mod.rs` → `crate::contract`：`detect_used_languages()` 读取 contract 来确定
      需要检查哪些语言/工具链。这是系统诊断功能，不属于 source 层的实现逻辑。
      做法：将 `detect_used_languages()` 和配套的 `build_tool_status_header()`、
      `build_language_sections()` 从 source/mod.rs 提取到独立的 `diagnostics` 模块或 main.rs。
- [ ] `src/release/mod.rs` 内联 contract 薄包装：`validate_version()`、`normalize_version()`、
      `precheck_version_changelog()`、`extract_notes()`、`confirm_release()`、`parse_github_repo()` ——
      这些要么是 contract 定义的规范（version），要么是 platform 的实现（github），不应放在 release 里。

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
