# AGENTS

## 特殊文件

- README：面向用户
- CONTRIBUTING：面向开发者
- AGENTS.md：AI 操作指南

## 设计决策

### code audit 不做 AST

`code audit` 是 DevOps 生命周期的门禁检查（红/绿），不是深度代码诊断工具。它只做文本级统计——不需要 tree-sitter 或其他语言 parser。

新指标的采纳门槛：**能否在不引入 parser 的前提下实现**。

AST 级分析（圈复杂度、长函数、未用变量等）归 `qtcloud-code review` 负责，它输出 STATUS.md 可被 `code audit` 聚合展示，但不依赖它做门禁判定。

### 网络 git 操作用 CLI 而非 git2

任何需要网络的 git 操作（push、fetch、pull、rebase、clone、delete remote tag 等）必须用 `std::process::Command::new("git")`，不要用 `git2` crate。

原因：`git2` 缺少 credential callback 配置，连接 GitHub HTTPS 时会报 "authentication required but no callback set"。系统 `git` 命令使用用户已配置的 credential helper（如 `gh auth setup-git`），不存在此问题。

`git2` 的使用范围：仅限纯本地操作（读配置、查日志、创建本地 tag、rev-parse 等）。
