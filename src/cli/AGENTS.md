# AGENTS

## 特殊文件

- README：面向用户
- CONTRIBUTING：面向开发者
- ROADMAP：版本规划
- TODO：具体待办

## 测试

```sh
cargo test                      # 全部测试
cargo test --test release       # 仅 release 集成测试
cargo test --test code          # 仅 code 集成测试
```

## 模块结构

```
src/
├── code/       # 业务层：纯抽象，不暴露 git 概念
├── git/        # 事实源底层：所有 git 操作
└── release/    # 发布子领域：publish
```

## CLI 命令

```bash
code sync [name]                # 同步组件（封装 fetch + push + pointer update）
code status [path] [--offline]  # 查看组件同步状态（Synced/PendingPush/PendingPull/Conflict）

release publish -v <version> [-y]  # 发布版本（-y 跳过确认）
```

### 规则

- `code sync`：`name` 省略时同步全部
- `code status`：路径默认为当前目录 `.`
- `release publish -y`：跳过用户确认
