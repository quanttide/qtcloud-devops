# STATUS

## 已知盲区

`qtcloud-devops release` 工具的预检查无法覆盖以下问题：

| 盲区 | 影响 | 建议检查方式 |
|------|------|------------|
| **依赖完整性** | pyproject.toml 漏写依赖，本地可运行但 pip 安装后崩溃 | `pip install -e .` 测试，或 CI 中 `uv sync --frozen` 验证 |
| **测试通过率** | 发布前测试套件是否全绿 | `pytest` 或 CI 状态 |
| **CHANGELOG 路径假设** | 默认 `CHANGELOG.md` 查找当前目录，包在子目录时需手动 `--changelog` | 从包目录（pyproject.toml 同层）执行，无需额外参数 |
| **当前目录假设** | `git remote`、`CHANGELOG.md` 都依赖 cwd，不在包目录运行行为不同 | 始终在 `pyproject.toml` 所在目录执行 |
| **PyPI 发布验证** | CLI 发完 GitHub Release 后不验证 PyPI 是否发布成功 | 在 CI 之外手动 `pip install` 验证，或在 workflow 中加 `pip install` 后 `--dry-run` 检查 |
| **Action secret** | publish workflow 依赖 `PYPI_API_TOKEN` secret，未配置时静默失败 | 发布后手动检查 CI run 状态 |
| **Node.js 版本** | actions/checkout@v4 和 setup-python@v5 使用 Node.js 20，2026-09 后将移除 | 关注 GitHub Actions 更新，及时升级 action 版本 |
| **Breaking change 标记** | CHANGELOG 有 breaking changes 时是否需要大版本 | 人工审查 CHANGELOG |
