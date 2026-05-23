# AGENTS

## ROADMAP

- 只记录**未完成**的事项，已完成的及时删除
- 按 P0-P3 优先级分组
- 基本假设放在末尾，不随版本迭代删除
- 新增需求先定优先级再放入对应分组

## 开发规范

### 单元测试覆盖率（红线 95%，黄线 98%）

所有公开发布的公共函数和模块必须达到一定的单元测试行覆盖率。

| 级别 | 阈值 | 含义 |
|------|------|------|
| 红线 | 95% | CI 拦截线，低于此值构建失败 |
| 黄线 | 98% | 目标值，低于此值应补充测试 |

命令：

```sh
# Python（红线 95%）
uv run pytest --cov=app/python --cov-report=term-missing --cov-fail-under=95

# Rust（黄线 98%，仅报告不失败）
cargo llvm-cov --html --output-dir target/coverage
```

**例外**（不纳入覆盖率统计）：
- `__init__.py`（空文件）
- 纯类型定义（`pydantic` Settings 等，无业务逻辑的 `__init__.py`）
- CI 配置、入口脚本

集成测试（`integrated_tests/`）不要求覆盖率，但单元测试（`tests/`）必须达标。

### 糟糕单元测试示例（禁止）

以下类型测试不会增加质量，只是徒增维护成本，AI 禁止生成：

**1. 测类型**

```python
# ❌ pydantic Settings 类型在导入时已校验，运行时测试无意义
def test_settings_is_basemodel():
    assert isinstance(settings, BaseSettings)
```

**2. 测恒真条件**

```python
# ❌ assert True 是凑行数
def test_version_string_not_empty():
    assert __version__ != ""
```

**3. 测框架行为**

```python
# ❌ clap/typer 的参数解析不需要你来测
def test_help_exits_with_zero():
    """这是集成测试的工作，不是单元测试的"""
```

**4. 用 mock 模拟简单计算**

```python
# ❌ mock 了所有输入和输出，实际测的是 mock 本身
mock_return = {"name": "test", "status": "Clean"}
monkeypatch.setattr("app.code.rust_call", lambda _: mock_return)
result = status(".")
assert result == mock_return  # 永远通过
```

**5. 测内部实现而非公开行为**

```python
# ❌ 测私有函数的调用次数，重构时必碎
def test_sync_calls_internal():
    spy = mocker.spy(editor, "_do_sync")
    editor.sync_to_parent("lib")
    assert spy.called_once
```

**原则**：单元测试只测公开接口的**输入→输出**逻辑，不测类型、不测框架、不测 mock。

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
- 所有命令通过 `app/qtcloud_devops_cli/code.py` 封装 Rust native 调用，错误处理在该层完成

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

tests/              ← Python 单元测试
integrated_tests/   ← Python 集成测试（需要真实 git 仓库等外部依赖）

## 测试覆盖率

### Python

```sh
uv run pytest --cov=app/python --cov-report=term-missing --cov-fail-under=100
```

依赖 `pytest-cov`（已配置在 `[dependency-groups] dev` 中）。

### Rust

使用 `cargo-llvm-cov`（推荐）：

```sh
cargo install cargo-llvm-cov
cargo llvm-cov --lcov --output-path target/coverage/lcov.info
# HTML 报告
cargo llvm-cov --html --output-dir target/coverage
```

也可使用 `cargo-tarpaulin`：

```sh
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```
