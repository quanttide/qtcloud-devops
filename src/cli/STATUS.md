# Code Scan Status

> 自动生成于 qtcloud-code scan，时间戳: 1779710789

## 汇总

| 级别 | 数量 |
|------|------|
| MUST   | 3 |
| SHOULD | 5 |
| MAY    | 9 |
| **Total** | **17** |

## 详情

- **../../apps/qtcloud-devops/src/cli/src/commands/code.rs** (4 项)
  - 🟡 **SHOULD** `rust-long-function` 26:1 — 函数 `sync_to_parent` 共 67 行
  - 🔵 **MAY** `rust-long-function` 109:1 — 函数 `retire_submodule` 共 45 行
  - 🔵 **MAY** `rust-long-function` 375:1 — 函数 `test_editor_retire_with_multiple_submodules` 共 36 行
  - 🟡 **SHOULD** `rust-long-function` 414:1 — 函数 `test_editor_sync_with_remote_push` 共 57 行
- **../../apps/qtcloud-devops/src/cli/src/commands/release.rs** (3 项)
  - 🟡 **SHOULD** `rust-long-function` 161:1 — 函数 `stage` 共 53 行
  - 🔵 **MAY** `rust-long-function` 218:1 — 函数 `publish` 共 50 行
  - 🔵 **MAY** `rust-long-function` 291:1 — 函数 `release_status` 共 37 行
- **../../apps/qtcloud-devops/src/cli/src/main.rs** (2 项)
  - 🔴 **MUST** `rust-long-function` 163:1 — 函数 `run_code` 共 97 行
  - 🔴 **MUST** `rust-long-function` 319:1 — 函数 `test_print_aggregate_with_variants` 共 111 行
- **../../apps/qtcloud-devops/src/cli/src/model/code.rs** (8 项)
  - 🔵 **MAY** `rust-long-function` 69:1 — 函数 `scan` 共 39 行
  - 🟡 **SHOULD** `rust-long-function` 110:1 — 函数 `scan_single_submodule` 共 57 行
  - 🔴 **MUST** `rust-long-function` 169:1 — 函数 `scan_submodule_remote_state` 共 89 行
  - 🔵 **MAY** `rust-long-function` 260:1 — 函数 `determine_submodule_status` 共 32 行
  - 🔵 **MAY** `rust-long-function` 429:1 — 函数 `setup_repo_with_submodule` 共 32 行
  - 🔵 **MAY** `rust-long-function` 701:1 — 函数 `test_aggregate_status_from_submodules` 共 47 行
  - 🔵 **MAY** `rust-long-function` 751:1 — 函数 `test_aggregate_status_all_variants` 共 32 行
  - 🟡 **SHOULD** `rust-long-function` 1058:1 — 函数 `test_scan_with_behind_remote` 共 55 行
