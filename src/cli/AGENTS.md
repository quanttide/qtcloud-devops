# AGENTS

## 特殊文件

- README：面向用户
- CONTRIBUTING：面向开发者

## 测试

```sh
cargo test                      # 全部测试
cargo test --test release       # 仅 release 集成测试
cargo test --test code          # 仅 code 集成测试
cargo test --test cli           # 仅 cli 集成测试
```

## 模块结构

```
src/
├── code/       # 业务层：纯抽象，不暴露 git 概念
├── git/        # 事实源底层：所有 git 操作
├── release/    # 发布子领域：publish + status
├── build.rs    # 构建状态查询
├── test.rs     # 测试状态查询
└── contract.rs # 契约适配层（委托 toolkit）
```

## CLI 命令

```bash
build status                        # 查看构建状态（CI、版本一致性）
test status                         # 查看测试状态（通过数、覆盖率）

code sync [name]                    # 同步组件（封装 fetch + push + pointer update）
code status [path] [--offline]      # 查看组件同步状态

release publish -v <version> [-y] [--registry <target>]  # 发布版本
release status                      # 查看发布状态（tag、CHANGELOG、GitHub Release）
```

### 规则

- `code sync`：`name` 省略时同步全部
- `code status`：路径默认为当前目录 `.`
- `release publish -y`：跳过用户确认
- `release publish --registry`：指定 CI 发布目标（crates / pypi / pubdev）
