use std::path::{Path, PathBuf};

use crate::contract;

// ═══════════════════════════════════════════════════════════════════════
// audit — 代码质量审计
// ═══════════════════════════════════════════════════════════════════════

pub fn audit(repo_path: &Path) {
    let c = contract::load(repo_path);

    let files = walk_scope_files(&c, repo_path);
    let scanned = scan_files(&files);

    let results: Vec<RuleResult> = vec![
        check_scope_dirs(&c, repo_path),
        check_todo_density(&scanned),
        check_import_count(&scanned),
        check_file_length(&scanned),
        check_lint(repo_path),
    ];

    print_report(&results);
}

// ── 结果类型 ──────────────────────────────────────────

pub struct RuleResult {
    pub name: &'static str,
    pub passed: bool,
    pub details: Vec<String>,
}

impl RuleResult {
    fn pass(name: &'static str, detail: String) -> Self {
        Self { name, passed: true, details: vec![detail] }
    }
    fn fail(name: &'static str, details: Vec<String>) -> Self {
        Self { name, passed: false, details }
    }
}

// ── 文件扫描模型 ──────────────────────────────────────

/// 扫描后的单个文件信息
struct ScannedFile {
    path: PathBuf,
    /// 总行数
    lines: usize,
    /// TODO/FIXME/HACK 位置列表
    todos: Vec<(usize, String)>,
    /// import / use 声明数
    imports: usize,
}

// ── 目录遍历 ──────────────────────────────────────────

const SRC_EXTENSIONS: &[&str] = &["rs", "py", "go", "ts", "tsx", "dart", "js", "jsx"];
const GENERATED_SUFFIXES: &[&str] = &[
    ".freezed.dart", ".g.dart", ".grpc.dart", ".pb.dart", ".pb.go",
];

/// 遍历所有 scope 目录，收集源码文件路径
fn walk_scope_files(c: &contract::Contract, repo_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for s in &c.scopes {
        collect_files(&repo_path.join(&s.dir), &mut files);
    }
    files
}

/// 递归收集目录下的源码文件
fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name != "target" && !name.starts_with('.') && name != "node_modules" {
                collect_files(&path, files);
            }
        } else if is_source_file(&path) {
            files.push(path);
        }
    }
}

/// 判断是否为可审计的源码文件
fn is_source_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !SRC_EXTENSIONS.contains(&ext) {
        return false;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if GENERATED_SUFFIXES.iter().any(|s| name.ends_with(s)) {
        return false;
    }
    true
}

// ── 文件扫描 ──────────────────────────────────────────

/// 批量扫描文件
fn scan_files(files: &[PathBuf]) -> Vec<ScannedFile> {
    files.iter().filter_map(|p| scan_one(p)).collect()
}

/// 扫描单个文件
fn scan_one(path: &Path) -> Option<ScannedFile> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines = content.lines().count();
    let todos: Vec<(usize, String)> = content
        .lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let t = l.trim().to_lowercase();
            if t.contains("todo") || t.contains("fixme") || t.contains("hack") {
                Some((i + 1, l.trim().to_string()))
            } else {
                None
            }
        })
        .collect();
    let imports = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("use ") || t.starts_with("import ")
        })
        .count();
    Some(ScannedFile { path: path.to_path_buf(), lines, todos, imports })
}

// ── 检查项 ────────────────────────────────────────────

/// 检查所有 scope 目录是否存在
fn check_scope_dirs(c: &contract::Contract, repo_path: &Path) -> RuleResult {
    let missing: Vec<String> = c
        .scopes
        .iter()
        .filter(|s| !repo_path.join(&s.dir).exists())
        .map(|s| format!("{}: 目录不存在 ({})", s.name, s.dir))
        .collect();
    if missing.is_empty() {
        RuleResult::pass("Scope 目录", format!("全部 {} 个 scope 存在", c.scopes.len()))
    } else {
        RuleResult::fail("Scope 目录", missing)
    }
}

/// 检查 TODO/FIXME 密度（阈值 5‰）
fn check_todo_density(scanned: &[ScannedFile]) -> RuleResult {
    let total_lines: usize = scanned.iter().map(|f| f.lines).sum();
    let total_markers: usize = scanned.iter().map(|f| f.todos.len()).sum();
    if total_lines == 0 {
        return RuleResult::pass("TODO/FIXME 密度", "无可扫描源码".to_string());
    }
    let density = total_markers as f64 / total_lines as f64 * 1000.0;
    if density < 5.0 {
        RuleResult::pass("TODO/FIXME 密度", format!("{} 处, 密度 {:.1}‰", total_markers, density))
    } else {
        let files_with_todos: Vec<String> = scanned
            .iter()
            .filter(|f| !f.todos.is_empty())
            .map(|f| format!("{} ({} 处)", f.path.display(), f.todos.len()))
            .collect();
        let mut details = vec![format!("{} 处, 密度 {:.1}‰（阈值 5‰）", total_markers, density)];
        details.extend(files_with_todos);
        RuleResult::fail("TODO/FIXME 密度", details)
    }
}

/// 检查 import 数（阈值 30 个每文件）
fn check_import_count(scanned: &[ScannedFile]) -> RuleResult {
    let high: Vec<String> = scanned
        .iter()
        .filter(|f| f.imports > 30)
        .map(|f| format!("{} ({} 个 import)", f.path.display(), f.imports))
        .collect();
    if high.is_empty() {
        RuleResult::pass("导入数", "全部文件 ≤ 30 个 import".to_string())
    } else {
        RuleResult::fail("导入数", high)
    }
}

/// 检查文件长度（阈值 500 行）
fn check_file_length(scanned: &[ScannedFile]) -> RuleResult {
    let long: Vec<String> = scanned
        .iter()
        .filter(|f| f.lines > 500)
        .map(|f| format!("{} ({} 行)", f.path.display(), f.lines))
        .collect();
    if long.is_empty() {
        RuleResult::pass("文件长度", "全部文件 ≤ 500 行".to_string())
    } else {
        RuleResult::fail("超长文件（阈值 500 行）", long)
    }
}

/// 调用外部 lint 命令
fn check_lint(repo_path: &Path) -> RuleResult {
    let langs = contract::detect_languages(repo_path);
    if langs.is_empty() {
        return RuleResult::pass("语法检查", "跳过（不支持的语言）".to_string());
    }
    let mut details = Vec::new();
    let mut all_ok = true;
    for lang in &langs {
        if let Some((cmd, args, label)) = lint_command(lang) {
            let ok = std::process::Command::new(cmd)
                .args(&args)
                .current_dir(repo_path)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !ok {
                all_ok = false;
            }
            details.push(format!("{}: {}", label, if ok { "✅" } else { "❌" }));
        }
    }
    if all_ok {
        RuleResult::pass("语法检查", details.join(", "))
    } else {
        RuleResult::fail("语法检查", details)
    }
}

fn lint_command(
    lang: &contract::Language,
) -> Option<(&'static str, Vec<&'static str>, &'static str)> {
    match lang {
        contract::Language::Rust => Some(("cargo", vec!["check", "--quiet"], "cargo check")),
        contract::Language::Python => Some(("uv", vec!["check"], "uv check")),
        contract::Language::TypeScript => Some(("npx", vec!["tsc", "--noEmit"], "tsc --noEmit")),
        _ => None,
    }
}

// ── 报告输出 ──────────────────────────────────────────

fn print_report(results: &[RuleResult]) {
    println!("代码审计\n{}", "-".repeat(50));
    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    for r in results {
        if r.passed {
            println!("  ✅ {}: {}", r.name, r.details.join("; "));
        } else {
            for d in &r.details {
                println!("  ❌ {}", d);
            }
        }
    }
    println!("\n{}", "-".repeat(50));
    if passed == total {
        println!("  ✅ 全部 {} 项检查通过", total);
    } else {
        println!("  ⚠ {}/{} 项通过", passed, total);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── lint_command ─────────────────────────────────────────────

    #[test]
    fn test_lint_command_rust() {
        let result = lint_command(&contract::Language::Rust);
        assert_eq!(result, Some(("cargo", vec!["check", "--quiet"], "cargo check")));
    }

    #[test]
    fn test_lint_command_python() {
        let result = lint_command(&contract::Language::Python);
        assert_eq!(result, Some(("uv", vec!["check"], "uv check")));
    }

    #[test]
    fn test_lint_command_typescript() {
        let result = lint_command(&contract::Language::TypeScript);
        assert_eq!(result, Some(("npx", vec!["tsc", "--noEmit"], "tsc --noEmit")));
    }

    #[test]
    fn test_lint_command_unknown() {
        let result = lint_command(&contract::Language::Unknown("x".into()));
        assert_eq!(result, None);
    }

    // ── is_source_file ───────────────────────────────────────────

    #[test]
    fn test_is_source_file_rs() {
        assert!(is_source_file(Path::new("foo.rs")));
    }

    #[test]
    fn test_is_source_file_py() {
        assert!(is_source_file(Path::new("foo.py")));
    }

    #[test]
    fn test_is_source_file_txt() {
        assert!(!is_source_file(Path::new("foo.txt")));
    }

    #[test]
    fn test_is_source_file_generated_dart() {
        assert!(!is_source_file(Path::new("foo.freezed.dart")));
        assert!(!is_source_file(Path::new("foo.g.dart")));
    }

    #[test]
    fn test_is_source_file_generated_go() {
        assert!(!is_source_file(Path::new("foo.pb.go")));
    }

    // ── scan_one ─────────────────────────────────────────────────

    #[test]
    fn test_scan_one_basic() {
        let dir = std::env::temp_dir().join("test_scan_one");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test.rs");
        std::fs::write(&path, "use std::collections;\nfn main() {}\n// TODO: fix this\n").ok();
        let result = scan_one(&path);
        assert!(result.is_some());
        let f = result.unwrap();
        assert_eq!(f.lines, 3);
        assert_eq!(f.imports, 1);
        assert_eq!(f.todos.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_scan_one_unreadable() {
        let result = scan_one(Path::new("/nonexistent/file.rs"));
        assert!(result.is_none());
    }

    // ── check_todo_density ───────────────────────────────────────

    #[test]
    fn test_check_todo_density_below_threshold() {
        let scanned = vec![ScannedFile {
            path: PathBuf::from("x.rs"),
            lines: 1000,
            todos: vec![],
            imports: 0,
        }];
        let result = check_todo_density(&scanned);
        assert!(result.passed);
    }

    #[test]
    fn test_check_todo_density_above_threshold() {
        // 6 markers / 1000 lines = 6‰ > 5‰ threshold
        let scanned = vec![ScannedFile {
            path: PathBuf::from("x.rs"),
            lines: 1000,
            todos: vec![
                (1, "TODO".into()), (2, "FIXME".into()), (3, "HACK".into()),
                (4, "TODO".into()), (5, "FIXME".into()), (6, "HACK".into()),
            ],
            imports: 0,
        }];
        let result = check_todo_density(&scanned);
        assert!(!result.passed);
    }

    #[test]
    fn test_check_todo_density_zero_lines() {
        let scanned = vec![];
        let result = check_todo_density(&scanned);
        assert!(result.passed);
    }

    // ── check_import_count ───────────────────────────────────────

    #[test]
    fn test_check_import_count_all_ok() {
        let scanned = vec![ScannedFile {
            path: PathBuf::from("x.rs"),
            lines: 10,
            todos: vec![],
            imports: 5,
        }];
        let result = check_import_count(&scanned);
        assert!(result.passed);
    }

    #[test]
    fn test_check_import_count_high() {
        let scanned = vec![ScannedFile {
            path: PathBuf::from("x.rs"),
            lines: 100,
            todos: vec![],
            imports: 35,
        }];
        let result = check_import_count(&scanned);
        assert!(!result.passed);
    }

    // ── check_file_length ────────────────────────────────────────

    #[test]
    fn test_check_file_length_all_ok() {
        let scanned = vec![ScannedFile {
            path: PathBuf::from("x.rs"),
            lines: 300,
            todos: vec![],
            imports: 0,
        }];
        let result = check_file_length(&scanned);
        assert!(result.passed);
    }

    #[test]
    fn test_check_file_length_long() {
        let scanned = vec![ScannedFile {
            path: PathBuf::from("x.rs"),
            lines: 600,
            todos: vec![],
            imports: 0,
        }];
        let result = check_file_length(&scanned);
        assert!(!result.passed);
    }

    // ── check_scope_dirs ─────────────────────────────────────────

    #[test]
    fn test_check_scope_dirs_all_exist() {
        let dir = std::env::temp_dir().join("test_scope_dirs_ok");
        std::fs::create_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir.join("core")).ok();
        std::fs::create_dir_all(&dir.join(".quanttide/devops")).ok();
        std::fs::write(
            &dir.join(".quanttide/devops/contract.yaml"),
            "stages:\nscopes:\n  core:\n    dir: core\n",
        )
        .ok();
        let c = contract::load(&dir);
        let result = check_scope_dirs(&c, &dir);
        assert!(result.passed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_check_scope_dirs_missing() {
        let dir = std::env::temp_dir().join("test_scope_dirs_missing");
        std::fs::create_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir.join(".quanttide/devops")).ok();
        std::fs::write(
            &dir.join(".quanttide/devops/contract.yaml"),
            "stages:\nscopes:\n  core:\n    dir: nonexistent\n",
        )
        .ok();
        let c = contract::load(&dir);
        let result = check_scope_dirs(&c, &dir);
        assert!(!result.passed);
        std::fs::remove_dir_all(&dir).ok();
    }
}
