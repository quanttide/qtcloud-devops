# ROADMAP

## 分层架构

```
CLI（qtcloud-devops-cli）        ← 当前阶段
GUI / Dashboard（待定）          ← 引入时机见下方
Server / API（待定）             ← 引入时机见下方
```

## 引入时机

### Server（服务端）

当出现以下任一场景时引入：

- 多人需要共享 release status（当前 journal 提交到 git 勉强可共享）
- 需要审批门禁（publish 前他人 approve）
- 跨项目发布编排（同时协调多仓库发布顺序）

### GUI（客户端）

当出现以下任一场景时引入：

- 非技术用户需要查看发布状态
- 需要审批操作（Web 上点通过）
- 需要可视化发布历史

## 当前阶段

纯 CLI + git 共享 journal。无服务端、无 GUI。

- 数据格式：`.quanttide/devops/release-journal.jsonl`（JSONL）已入 git
- 分发：PyPI（pip）/ crates.io（cargo）/ GitHub Releases
- 文档：GitHub Pages（https://quanttide.github.io/qtcloud-devops/）
