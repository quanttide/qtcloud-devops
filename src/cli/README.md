# 量潮DevOps云命令行工具(`qtcloud-devops-cli`)

## 安装

### 前置依赖

- **Rust 工具链**：`rustup` + `cargo`
- **libgit2**：`sudo apt install libgit2-dev`（Ubuntu）或 `brew install libgit2`（macOS）

### 从 crates.io 安装

```bash
cargo install qtcloud-devops-cli
```

### 从源码安装

```bash
git clone https://github.com/quanttide/qtcloud-devops.git
cd apps/qtcloud-devops/src/cli
cargo build --release
target/release/qtcloud-devops --help
```

## 用法

```bash
# 构建状态
qtcloud-devops build status

# 测试状态
qtcloud-devops test status

# 组件同步状态
qtcloud-devops code status [path] [--offline]
qtcloud-devops code sync [name]

# 发布
qtcloud-devops release publish -v <version> [-y] [--registry <target>]
qtcloud-devops release status
```

### 规则

- `code sync`：`name` 省略时同步全部
- `code status`：路径默认为当前目录 `.`
- `release publish -y`：跳过用户确认
- `release publish --registry`：指定 CI 发布目标（crates / pypi / pubdev）
