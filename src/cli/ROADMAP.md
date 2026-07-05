# ROADMAP — qtcloud-devops-cli

## v0.9.3 — 自动版本检测

为 `release publish` 增加自动判断版本的能力。`-v` 变为可选，省略时自动检测版本号。

### 用法

```bash
# 自动检测 + 一键发布
qtcloud-devops release publish -y

# 输出示例：
# 📌 项目类型: code
# 📌 scope: cli
# 📦 最新标签: cli/v0.9.2
# 📝 提交数: 3
#    • feat: add force publish
#    • fix: sync Cargo.lock
#    • chore: update deps
# 🧠 LLM 决策: 有 feat，minor 走 rc 预发布
# 🔮 建议版本: cli/v0.10.0-rc.1
# ✅ 已发布 cli/v0.10.0-rc.1

# 仅预览，不执行
qtcloud-devops release publish --dry-run

# 手动指定（不变）
qtcloud-devops release publish -v cli/v0.9.3 -y
```

### 任务分解

1. **基础设施移植** — 将实验室 `detect` 的 infrastructure 函数移植到 CLI：
   - `collect_tags_with_scope()` / `parse_tag()` / `parse_version()`
   - `build_version()` / `detect_scope()` / `detect_project_type()`
   - `llm_decide()` / `fallback_heuristic()`

2. **`-v` 变为可选** — 在 `ReleaseAction::Publish` 中 `version` 改为 `Option<String>`：
   - 有 `-v` → 直接使用指定版本（当前行为）
   - 无 `-v` → 调用 detect 逻辑自动推断，打印建议版本后进入确认流程

3. **`--dry-run`** — 新增 `dry_run: bool` 参数：
   - 仅在 `publish()` 入口提前返回，打印所有信息但无副作用
   - 有 `-v` 时也支持 `--dry-run`（只展示不执行）

4. **交互确认** — 复用已有的 `confirm_release()`：
   - 自动检测版本后打印建议版本
   - 有 `-y` 跳过确认，无 `-y` 走现有确认流程

### 技术要点

- `detect` 逻辑放在 `src/release/detect.rs` 模块
- 复用已有的 `git2` 和 `quanttide-agent` 依赖，无需新增
- 与实验室 `detect` 保持同步，后续迭代合并到 CLI
- `--dry-run` 在入口 return，不触动任何 git/config 操作

### 验收标准

- [ ] 无 `-v` 时自动检测版本号并发布
- [ ] 有 `-v` 时行为不变（兼容现有用法）
- [ ] `--dry-run` 打印所有信息但不执行任何操作
- [ ] 自动检测输出包含项目类型、scope、提交统计
- [ ] 有 LLM 时使用 LLM 决策，无 LLM 时回退到启发式规则
- [ ] 多 scope 仓库正确检测当前 scope 的版本
- [ ] 交互确认（有 `-y` 时跳过）
