# build

## 职责

围绕构建的命令。运行本地构建，输出结果摘要。

## 设计

运行项目本地的构建命令。

| 目标 | 命令 | 路径 |
|------|------|------|
| Rust 构建 | `cargo build` | `src/` |
| Python 构建 | `uv build` | 项目根目录 |
| sdist | `uv build --sdist` | 项目根目录 |

### 输出

- 构建结果（成功/失败）
- 错误信息摘要
- 构建产物路径

### 过滤

| 参数 | 说明 |
|------|------|
| `--rust` | 仅 Rust 构建 |
| `--python` | 仅 Python 构建 |
| `--release` | release 模式构建 |

### 风格

与 `release` 命令组一致：Rust 实现、原子操作。
