# STATUS

## 已知盲区

`qtcloud-devops release` 工具的预检查无法覆盖以下问题：

| 盲区 | 影响 | 建议检查方式 |
|------|------|------------|
| **依赖完整性** | pyproject.toml 漏写依赖（如 typer、pydantic-settings），本地可运行但 pip 安装后崩溃 | `pip install -e .` 测试，或 CI 中 `uv sync --frozen` 验证 |
| **uv.lock 同步** | pyproject.toml 版本号改后 uv.lock 未同步 | `uv lock` 更新，确保 `uv.lock` 中的 version 与 pyproject.toml 一致 |
| **子模块引用** | 主仓库子模块指针未随子仓库更新 | `git submodule status` 检查 |
| **测试通过率** | 发布前测试套件是否全绿 | `pytest` 或 CI 状态 |
| **CI 发布流水线** | 仓库是否有 publish workflow | 检查 `.github/workflows/` |
| **Breaking change 标记** | CHANGELOG 有 breaking changes 时是否需要大版本 | 人工审查 CHANGELOG |
