pub(crate) mod audit;
pub(crate) mod run;
pub(crate) mod status;
pub(crate) mod coverage;
pub(crate) mod summary;

pub use audit::audit;
pub use run::run;
pub use status::{status, status_to};
pub use summary::clear_cache;

/// 测试结果汇总。
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TestSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
}

/// 覆盖率数据。
#[derive(Debug, Default)]
pub struct Coverage {
    pub percentage: f64,
    pub threshold: f64,
}

impl Coverage {
    pub fn met(&self) -> bool {
        self.percentage >= self.threshold
    }
}

/// I/O 函数模式：这些名称的函数不要求单元测试（集成测试覆盖即可）。
/// 注意：纯函数（parse_/determine_/build_/fmt_/map_/normalize_/apply_/extract_/is_）不应在此列表。
const IO_FN_PATTERNS: &[&str] = &[
    "status", "status_to", "run", "run_direct", "run_scoped",
    "sync", "sync_all", "sync_to_parent", "sync_all_to_parent",
    "publish", "scan", "scan_offline",
    "push_submodule", "push_parent", "fetch_submodule", "rebase_submodule",
    "check_command", "check_syntax", "check_ci", "check_deps",
    "clear_cache", "save_test_summary",
    "ensure_", "delete_", "create_",
    "git_output", "llm_decide", "llm_changelog", "edit_llm",
    "detect_version", "detect_single_scope", "detect_project_type",
    "resolve_roadmap_path",
    "print_status", "print_status_to", "print_scope_audit",
    "collect_git_log", "collect_tags_with_scope", "collect_test_summary_from_run",
    "load_contract_scopes", "load_scopes_map",
    "apply_rule_fixes",
    "collect_rs_files", "collect_test_fns", "collect_pub_fns", "collect_error_variants",
];

pub(crate) fn is_io_fn(name: &str) -> bool {
    IO_FN_PATTERNS.iter().any(|p| {
        if p.ends_with('_') {
            name.starts_with(p)
        } else {
            name == *p
        }
    })
}

/// 质量审计结果。
#[derive(Debug, Default)]
pub struct AuditReport {
    pub total_tests: usize,
    pub total_pub_fns: usize,
    pub pure_pub_fns: usize,
    pub tested_pub_fns: usize,
    pub error_variants: usize,
    pub tested_variants: usize,
    pub uncovered_fns: Vec<(String, String)>,
    pub uncovered_variants: Vec<String>,
    pub coverage_pct: f64,
    pub coverage_threshold: f64,
    pub gates_met: bool,
}

#[cfg(test)]
mod tests;
