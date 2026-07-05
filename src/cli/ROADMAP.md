# ROADMAP — qtcloud-devops-cli

## v0.9.3 — 自动版本检测

为命令行增加自动判断版本的能力，将实验室 `detect` 原型集成到 CLI 中。

### 目标

`qtcloud-devops release detect` 自动推断下一个版本号，作为 `release publish` 的前置步骤。

### `release detect` 子命令

```bash
# 自动检测并输出建议版本号
qtcloud-devops release detect

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
```

### 任务分解

1. **基础设施移植** — 将实验室 `detect` 的 infrastructure 函数移植到 CLI：
   - `collect_tags_with_scope()` — tag 收集 + semver 排序
   - `parse_tag()` / `parse_version()` — 版本号解析
   - `build_version()` — 根据决策构建版本字符串
   - `detect_scope()` — 从 changed files 推断 scope
   - `detect_project_type()` — 检测 code/docs 项目类型

2. **LLM 决策集成** — 使用已存在的 `quanttide-agent` 依赖：
   - 复用 `Settings::from_env()` + `LLM` 客户端
   - prompt 与实验室一致（项目类型、约束规则、输出格式）
   - 无 LLM 时回退到启发式规则（`fallback_heuristic`）

3. **`release detect` 子命令** — 在 `main.rs` 的 `ReleaseAction` 枚举中新增：
   ```rust
   /// 自动检测建议版本号
   Detect,
   ```

4. **`release publish --auto`**（可选延伸）— 自动检测 + 一键发布：
   ```bash
   qtcloud-devops release publish --auto
   ```
   等效于 `detect` + `publish -v <detected-version> -y`

### 技术要点

- 复用已有的 `git2` 和 `quanttide-agent` 依赖，无需新增
- `detect` 逻辑放在 `src/release/detect.rs` 模块
- 与实验室 `detect` 保持同步，后续实验室的迭代再合并到 CLI

### 验收标准

- [ ] `qtcloud-devops release detect` 在有 tag 的仓库输出正确版本建议
- [ ] `qtcloud-devops release detect` 在无 tag 的新仓库输出 `v0.1.0`
- [ ] 多 scope 时分别输出各 scope 的建议（如 `python/v0.1.5` + `rust/v0.2.0`）
- [ ] 无 LLM 时回退到启发式规则正常工作
- [ ] 项目类型（code/docs）显示在输出中
- [ ] 已有测试覆盖
