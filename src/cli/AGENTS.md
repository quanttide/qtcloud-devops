# AGENTS

## ROADMAP

- 只记录**未完成**的事项，已完成的及时删除
- 按 P0-P3 优先级分组
- 基本假设放在末尾，不随版本迭代删除
- 新增需求先定优先级再放入对应分组

## 提交消息

- `feat:` — 新功能
- `chore:` — 版本号变更、配置更新
- `docs:` — 文档更新
- `fix:` — 修 bug
- `test:` — 测试

## CLI 设计规则

### `code` 子命令行为

```
qtcloud-devops code status [path]                # 三路 commit 比对 + 聚合统计
qtcloud-devops code sync [name] [--repo path]    # 同步子模块指针到父仓库
qtcloud-devops code retire <name> [--repo path]  # 退役子模块
```

### 规则

- `status`：路径默认为当前目录 `.`
- `sync`：`name` 省略时同步全部子模块
- `retire`：`name` 为必填参数
- 所有命令通过 `app/code.py` 封装 Rust native 调用，错误处理在该层完成

### release 命令行为

```
qtcloud-devops release --version v0.1.0                # 标签 + GitHub Release（默认）
qtcloud-devops release --version v0.1.0 --tag-only      # 仅标签
qtcloud-devops release --version v0.1.0 --release-only  # 仅 GitHub Release
```

### 规则

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

## 测试目录结构

```
tests/
├── python/             # Python 单元测试
└── rust/               # Rust 集成测试（通过 Cargo.toml [[test]] 注册）
integrated_tests/       # Python 集成测试（需要真实 git 仓库等外部依赖）
```
