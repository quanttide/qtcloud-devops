# 架构：status / audit / action

所有命令按输入来源和副作用分为三类：

| 类别 | 读 source | 读 contract | 写（副作用） |
|------|-----------|-------------|-------------|
| **status** | ✅ 当前状态 | ❌ | ❌ |
| **audit** | ✅ 源码/系统 | ✅ 标准/门禁 | ❌ |
| **action** | ✅ | ✅ | ✅ 执行 |

## status — 事实

只读 source（git、文件系统、缓存），不读 contract，不做判定。

```
test status     测试缓存中的通过数/覆盖率
release status  git 标签/未发布提交数
build status    本地编译/CI/依赖状态
contract status  契约文件内容
doctor status   系统命令安装状态
code status     子模块同步状态
```

输出：数据。无 ❌ 图标，无门禁判定。

## audit — 标准

读 source + 读 contract，对照标准判定 pass/fail。

```
test audit      函数覆盖率/错误变体覆盖 vs 契约质量门禁
release audit   版本/配置/CHANGELOG/工作区/标签/远程/GH Release 6+1 项
```

输出：✅/❌。门禁不达标退出码 1。

## action — 执行

读 source + 读 contract + 写。

```
code sync       推送子模块变更
test run        执行测试 + 覆盖率
release publish 创建 tag + 推送 + GitHub Release
```

输出：执行结果。

## 格栅：scopes × stages

命令按 stage（行）和操作（列）组织：

| stage | status | audit | action |
|-------|--------|-------|--------|
| **code** | `code status` | — | `code sync` |
| **build** | `build status` | — | — |
| **test** | `test status` | `test audit` | `test run` |
| **release** | `release status` | `release audit` | `release publish` |
| *跨 stage* | `contract status` | — | — |
| *跨 stage* | `doctor status` | — | — |

每个 `(scope, stage)` 格子的输出由 contract.yaml 中的 scope 定义控制。
空单元格表示尚未实现。

## 不变量

1. **status 不加 audit** — `release status` 曾混入 ✅/❌ 判定，已清理（-606 行）
2. **audit 不加 status** — `test audit` 只输出判定，不展示原始计数
3. **action 可调用 audit** — `release publish` 内部已含预检，但独立 `release audit` 供 CI 提前拦截
