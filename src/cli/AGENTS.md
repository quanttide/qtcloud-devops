# AGENTS — qtcloud-devops-cli (Rust Crate)

## 测试

```sh
cargo test                      # 全部 175 测试
cargo test --test release       # 仅 release 集成测试
cargo test --test code          # 仅 code 集成测试
```

## 架构决策

### 单 crate 结构
- 当前保持单 crate（无 workspace），model crate 提取延后
- 触发条件：出现第二个 Rust 消费者

### PyO3
- `pyo3` 是 optional feature（`python`），默认不启用
- Wheel 构建由 `maturin` 处理，`pyproject.toml` 在子模组根目录

## 模块结构

```
src/
├── main.rs           # CLI 入口，clap Parser + Subcommand 分发
├── lib.rs            # 公开 commands 和 model 模块
├── commands/
│   ├── mod.rs
│   ├── release.rs    # stage/publish/retire/status + 测试
│   └── code.rs       # status/sync/retire + 测试
├── model/
│   ├── mod.rs
│   ├── release.rs    # ReleaseStatus, Storage, journal
│   └── code.rs       # RepoState, Submodule, SubmoduleStatus
└── python.rs         # PyO3 绑定（未维护）
```

## 依赖管理

- 新增依赖需评估编译时间影响，优先选纯 Rust 替代
- `regex` 已用于版本号校验，避免重复造轮子
- 避免引入 async runtime（tokio/async-std），项目为同步 CLI
