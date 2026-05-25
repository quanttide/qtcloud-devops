# Code Scan Status

> 自动生成于 qtcloud-code scan，时间戳: 1779711208

## 汇总

| 级别 | 数量 |
|------|------|
| MUST   | 0 |
| SHOULD | 4 |
| MAY    | 12 |
| **Total** | **16** |

## 详情

- **../../apps/qtcloud-devops/src/cli/src/commands/code.rs** (3 项)
  - 🔵 **MAY** `rust-long-function` 138:1 — 函数 `retire_submodule` 共 45 行
  - 🔵 **MAY** `rust-long-function` 404:1 — 函数 `test_editor_retire_with_multiple_submodules` 共 36 行
  - 🟡 **SHOULD** `rust-long-function` 443:1 — 函数 `test_editor_sync_with_remote_push` 共 57 行
- **../../apps/qtcloud-devops/src/cli/src/commands/release.rs** (3 项)
  - 🟡 **SHOULD** `rust-long-function` 161:1 — 函数 `stage` 共 53 行
  - 🔵 **MAY** `rust-long-function` 218:1 — 函数 `publish` 共 50 行
  - 🔵 **MAY** `rust-long-function` 291:1 — 函数 `release_status` 共 37 行
- **../../apps/qtcloud-devops/src/cli/src/main.rs** (2 项)
  - 🔵 **MAY** `rust-long-function` 178:1 — 函数 `run_code_status` 共 36 行
  - 🔵 **MAY** `rust-long-function` 303:1 — 函数 `test_print_aggregate_with_variants` 共 45 行
- **../../apps/qtcloud-devops/src/cli/src/model/code.rs** (8 项)
  - 🔵 **MAY** `rust-long-function` 69:1 — 函数 `scan` 共 39 行
  - 🟡 **SHOULD** `rust-long-function` 110:1 — 函数 `scan_single_submodule` 共 57 行
  - 🔵 **MAY** `rust-long-function` 250:1 — 函数 `compute_submodule_diff` 共 32 行
  - 🔵 **MAY** `rust-long-function` 284:1 — 函数 `determine_submodule_status` 共 32 行
  - 🔵 **MAY** `rust-long-function` 453:1 — 函数 `setup_repo_with_submodule` 共 32 行
  - 🔵 **MAY** `rust-long-function` 725:1 — 函数 `test_aggregate_status_from_submodules` 共 47 行
  - 🔵 **MAY** `rust-long-function` 775:1 — 函数 `test_aggregate_status_all_variants` 共 32 行
  - 🟡 **SHOULD** `rust-long-function` 1082:1 — 函数 `test_scan_with_behind_remote` 共 55 行
