# AGENTS

## 包信息

- **包名**: `qtcloud-devops-cli`
- **语言**: Python 3.12+
- **包管理器**: uv
- **定位**: DevOps CLI — 发布管理、契约检查与工作流自动化

## 项目结构

```
src/cli/
├── app/
│   ├── __init__.py
│   ├── cli.py          # Typer CLI 入口
│   ├── config.py       # pydantic-settings 配置
│   ├── contract.py     # 契约加载与查询
│   └── release.py      # 发布 Release 逻辑
├── tests/
│   ├── __init__.py
│   ├── conftest.py
│   ├── test_cli_commands.py
│   ├── test_release.py
│   └── test_contract.py
├── pyproject.toml
├── AGENTS.md
├── CHANGELOG.md
├── ROADMAP.md
└── .gitignore
```

## 提交消息

- `feat:` — 新功能
- `chore:` — 版本号变更、配置更新
- `docs:` — 文档更新
- `fix:` — 修 bug
- `test:` — 测试

## 发布流程

1. **更新版本号** → 改 `pyproject.toml`
2. **写 CHANGELOG** → 更新 `CHANGELOG.md`
3. **提交** → `chore: bump qtcloud-devops-cli to vX.Y.Z`
4. **打 tag** → `cli/vX.Y.Z`
5. **推送** → CI 自动发布

## 命名约定

- 包名（PyPI）: `qtcloud-devops-cli`
- 导入名: `app.*`
- 仓库 tag 前缀: `cli/`
