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

### Step 2：提交

```bash
cd apps/qtcloud-devops/src/cli
git add -A
git commit -m "chore: bump to v0.3.0"
git push origin main
```

### Step 3：发布

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

### Step 4：验证

检查两个渠道的发布结果：

```bash
cargo search qtcloud-devops-cli --registry crates-io
# 应显示 qtcloud-devops-cli = "0.3.0"

pip install qtcloud-devops-cli==0.3.0
qtcloud-devops --version
# 应输出 qtcloud-devops 0.3.0
```
