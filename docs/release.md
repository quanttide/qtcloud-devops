# 发布教程

以 v0.3.0 为例，展示完整发布流程。

## 发布流程

### Step 1：版本号升级

三个文件的版本号必须一致：

```bash
# Cargo.toml
version = "0.3.0"

# pyproject.toml
version = "0.3.0"

# CHANGELOG.md
## [0.3.0] - 2026-05-24
```

### Step 2：本地预验证

```bash
cd apps/qtcloud-devops/src/cli
./scripts/preflight.sh
```

preflight 会依次执行：

1. `cargo build --release` — 确认编译通过
2. `cargo test` — 确认测试通过
3. `cargo publish --dry-run --registry crates-io` — 确认 crates.io 发布可行
4. `maturin build --release --out dist --auditwheel skip` — 确认 wheel 构建可行

全部通过才能进入下一步。

### Step 3：提交

```bash
cd apps/qtcloud-devops/src/cli
git add -A
git commit -m "chore: bump to v0.3.0"
git push origin main
```

commit 前 review 变更清单，确保没有捡到多余文件。

### Step 4：发布

确认本地 commit 已推送后，使用 CLI 发布自身：

```bash
cd apps/qtcloud-devops/src/cli
cargo build                              # 确保二进制是最新版本
qtcloud-devops stage -v cli/v0.3.0       # 标记版本
qtcloud-devops publish -v cli/v0.3.0 -y  # 打 tag + GitHub Release
```

`stage` 校验版本号格式，`publish` 执行 tag 创建、推送、GitHub Release。

publish 成功后会自动触发 GitHub Actions：

```
release published
    → build-cli（三平台构建 + wheel 构建）
        → publish-crate（crates.io）
        → publish-pypi（PyPI）
```

### Step 5：验证

检查两个渠道的发布结果：

```bash
cargo search qtcloud-devops-cli --registry crates-io
# 应显示 qtcloud-devops-cli = "0.3.0"

pip install qtcloud-devops-cli==0.3.0
qtcloud-devops --version
# 应输出 qtcloud-devops 0.3.0
```

## 回滚

如果发布过程中断（如 CI 失败），递增 rc 序号重新发布：

```bash
# 不删除已有 release
# 修复问题后走相同流程发布 rc.N+1
git tag -d cli/v0.3.0-rc.N                # 仅删本地 tag
# 修复问题后重新发布
```

## 纪律

1. **AI 禁止直接 publish** — git 操作止于 commit && push
2. **发布前跑 preflight** — 本地拦截问题
3. **一次 rc 验证所有** — 操作系统 + 注册源 + 元数据，不分散
