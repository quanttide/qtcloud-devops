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

## CLI 设计规则

### release 命令行为

```
qtcloud-devops release --version v0.1.0                # 标签 + GitHub Release（默认）
qtcloud-devops release --version v0.1.0 --tag-only      # 仅标签
qtcloud-devops release --version v0.1.0 --release-only  # 仅 GitHub Release
```

**规则：**

- **默认** = 标签 + GitHub Release（仓库从 git remote 自动检测）
- `--tag-only` 和 `--release-only` 互斥
- tag 是否已存在的处理：
  - `--release-only`：tag **必须**存在，否则拒绝
  - 默认 / `--tag-only`：tag 存在则跳过创建，不影响后续
- `--repo` 参数**不存在**，仓库名通过 `get_remote_repo()` 从 `git remote get-url origin` 解析
- 发布后**不验证** GitHub Release（`verify_release` 函数未使用）
- 创建标签失败：返回错误码 1
- 推送标签失败：自动回滚本地标签
- GitHub Release 创建失败：若之前创建了标签则自动回滚
