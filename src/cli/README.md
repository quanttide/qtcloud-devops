# 量潮DevOps云命令行工具(`qtcloud-devops-cli`)

## 安装

### 前置依赖

- **Rust 工具链**：`rustup` + `cargo`

### 从 crates.io 安装

```bash
cargo install qtcloud-devops-cli
```

### 从源码安装

```bash
git clone https://github.com/quanttide/qtcloud-devops.git
cd apps/qtcloud-devops/src/cli
cargo install --path .
```

## 用法

```bash
# 概览状态
qtcloud-devops status

# 构建状态
qtcloud-devops build status

# 测试
qtcloud-devops test status          # 查看测试状态（读缓存）
qtcloud-devops test run              # 运行测试 + 覆盖率
qtcloud-devops test clean            # 清理缓存的测试结果

# 组件同步管理
qtcloud-devops code status [path] [--offline]
qtcloud-devops code sync [name] [--dry-run]

# 发布
qtcloud-devops release publish                       # 自动检测版本 + 发布
qtcloud-devops release publish -v cli/v0.10.0-rc.1   # 指定版本
qtcloud-devops release publish --dry-run              # 仅预览，不执行
qtcloud-devops release publish -y                     # 跳过确认
qtcloud-devops release status                         # 查看发布状态

# 规划管理
qtcloud-devops plan status [scope]    # 查看 scope 规划进度
qtcloud-devops plan clean [scope]     # 删除已完成条目
qtcloud-devops plan doctor [scope]    # 修复格式问题

# 契约管理
qtcloud-devops contract status        # 查看契约状态

# 系统诊断
qtcloud-devops doctor status          # 检查外部依赖
```

### 规则

- `code sync`：`name` 省略时同步全部
- `code status`：路径默认为当前目录 `.`
- `release publish`：`-v` 省略时自动检测版本号
- `release publish --dry-run`：仅预览不执行
- `release publish -y`：跳过用户确认
- `release publish --registry`：指定 CI 发布目标（crates / pypi / pubdev）
