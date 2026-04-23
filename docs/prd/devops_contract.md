# DevOps 契约

qtcloud-devops 仓库的约束性声明。

## Git 约束

- 禁止直接 push 到 main（必须 PR 合并）
- 禁止 force push 到 main
- 禁止空提交
- 必须使用 Conventional Commits

## 子模块约束

- 禁止添加空仓库为子模块
- 子模块必须指定 branch
- 禁止在 detached HEAD 状态提交
- 禁止跳过子模块引用更新

## 发布约束

- 禁止跳过 CHANGELOG 更新
- 禁止重复版本号
- 禁止在工作区有变更时发布