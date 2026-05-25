# 发布教程

以 v0.4.1 为例，展示完整发布流程。

## 发布流程

### Step 1：版本号升级

三个文件的版本号必须一致：

```bash
# Cargo.toml
version = "0.4.1"

# pyproject.toml
version = "0.4.1"

# CHANGELOG.md
## [0.4.1] - 2026-05-25
```

### Step 2：提交

```bash
cd apps/qtcloud-devops/src/cli
git add -A
git commit -m "chore: bump to v0.4.1"
git push origin main
```

### Step 3：发布

```bash
cd apps/qtcloud-devops/src/cli
cargo build
qtcloud-devops release publish -v cli/v0.4.1 -y
```

正式版不需要 `release stage`，直接 `release publish`。预发布版本才需要先 `release stage`。

### Step 4：验证

```bash
cargo search qtcloud-devops-cli --registry crates-io
# 应显示 qtcloud-devops-cli = "0.4.1"

pip install qtcloud-devops-cli==0.4.1
qtcloud-devops --version
# 应输出 qtcloud-devops 0.4.1
```
