use std::path::{Path, PathBuf};

use crate::contract;

// ═══════════════════════════════════════════════════════════════════════
// audit — 代码质量审计
// ═══════════════════════════════════════════════════════════════════════

/// 运行审计并打印报告。
pub fn audit(repo_path: &Path) {
    let results = run_audit(repo_path);
    print_report(&results);
}

/// 运行审计并以 JSON 格式输出（供 plan todo-from-audit 消费）。
pub fn audit_json(repo_path: &Path) -> String {
    let results = run_audit(repo_path);
    let plan = to_audit_plan(&results);
    serde_json::to_string_pretty(&plan).unwrap_or_default()
}

/// 运行审计，返回所有检查结果。
pub fn run_audit(repo_path: &Path) -> Vec<RuleResult> {
    let c = contract::load(repo_path);
    let files = walk_scope_files(&c, repo_path);
    let scanned = scan_files(&files);
    vec![
        check_scope_dirs(&c, repo_path),
        check_todo_density(&scanned),
        check_fn_length(&scanned),
        check_api_docs(&scanned),
        check_complexity(&scanned),
        check_import_count(&scanned),
        check_file_length(&scanned),
        check_mod_doc(&scanned),
        check_lint(repo_path),
    ]
}

// ── 结果类型 ──────────────────────────────────────────

pub struct RuleResult {
    pub name: &'static str,
    pub passed: bool,
    pub details: Vec<String>,
}

impl RuleResult {
    fn pass(name: &'static str, detail: String) -> Self {
        Self {
            name,
            passed: true,
            details: vec![detail],
        }
    }
    fn fail(name: &'static str, details: Vec<String>) -> Self {
        Self {
            name,
            passed: false,
            details,
        }
    }
}

// ── 审计计划（JSON 输出，供 plan todo-from-audit 消费） ──

/// 审计输出的 JSON 顶层结构。
#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuditPlan {
    pub source: String,
    pub source_label: String,
    pub entries: Vec<AuditPlanPriority>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuditPlanPriority {
    pub priority: String,
    pub items: Vec<AuditPlanItem>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuditPlanItem {
    pub check: String,
    pub file: String,
    pub detail: String,
}

/// 从 RuleResult 列表转换为 AuditPlan。
fn to_audit_plan(results: &[RuleResult]) -> AuditPlan {
    let mut must = Vec::new();
    let mut should = Vec::new();
    let mut may = Vec::new();

    for r in results {
        if r.passed {
            continue;
        }
        for d in &r.details {
            let (file, detail) = split_detail(d);
            let item = AuditPlanItem {
                check: r.name.to_string(),
                file,
                detail,
            };
            match classify_priority(r.name, d) {
                "MUST" => must.push(item),
                "SHOULD" => should.push(item),
                _ => may.push(item),
            }
        }
    }

    let mut entries = Vec::new();
    if !must.is_empty() {
        entries.push(AuditPlanPriority {
            priority: "MUST".to_string(),
            items: must,
        });
    }
    if !should.is_empty() {
        entries.push(AuditPlanPriority {
            priority: "SHOULD".to_string(),
            items: should,
        });
    }
    if !may.is_empty() {
        entries.push(AuditPlanPriority {
            priority: "MAY".to_string(),
            items: may,
        });
    }

    AuditPlan {
        source: "code-audit".to_string(),
        source_label: "代码审计".to_string(),
        entries,
    }
}

/// 从 `file: detail` 格式中拆出 file 和 detail。若不含 `: ` 则 file 为空。
fn split_detail(d: &str) -> (String, String) {
    if let Some(pos) = d.find(": ") {
        (d[..pos].to_string(), d[pos + 2..].to_string())
    } else {
        (String::new(), d.to_string())
    }
}

/// 根据检查名称和详情内容判断优先级。
fn classify_priority(check_name: &str, detail: &str) -> &'static str {
    match check_name {
        "Scope 目录" | "语法检查" => "MUST",
        "函数长度" if detail.contains("大幅") => "MUST",
        "结构复杂度" => "MUST",
        "函数长度" => "SHOULD",
        "API 文档覆盖率" => "SHOULD",
        "导入数" => "SHOULD",
        "TODO/FIXME 密度" => "MAY",
        "文件长度" => "MAY",
        "模块文档" => "MAY",
        _ => "SHOULD",
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
    /// 超长函数列表 (行数, 名称)
    long_fns: Vec<(usize, String)>,
    /// 缺少文档注释的 pub 函数
    missing_docs: Vec<String>,
    /// 最大嵌套深度
    max_nesting: usize,
    /// 高圈复杂度函数列表 (复杂度, 名称)
    high_complexity: Vec<(usize, String)>,
    /// 是否有模块级文档（//!）
    has_mod_doc: bool,
}

// ── 目录遍历 ──────────────────────────────────────────

const SRC_EXTENSIONS: &[&str] = &["rs", "py", "go", "ts", "tsx", "dart", "js", "jsx"];
const GENERATED_SUFFIXES: &[&str] = &[
    ".freezed.dart",
    ".g.dart",
    ".grpc.dart",
    ".pb.dart",
    ".pb.go",
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
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let todos = scan_todo_markers(&lines);
    let imports = count_imports(&lines);
    let fn_starts = find_fn_positions(&lines);
    let long_fns = find_long_fns(&lines, &fn_starts, total_lines);
    let missing_docs = find_missing_doc_comments(&lines);
    let max_nesting = compute_max_nesting(&lines);
    let high_complexity = extract_high_complexity(&lines);
    let has_mod_doc = lines.iter().take(10).any(|l| l.trim().starts_with("//!"));

    Some(ScannedFile {
        path: path.to_path_buf(),
        lines: total_lines,
        todos,
        imports,
        long_fns,
        missing_docs,
        max_nesting,
        high_complexity,
        has_mod_doc,
    })
}

fn scan_todo_markers(lines: &[&str]) -> Vec<(usize, String)> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let t = l.trim().to_lowercase();
            if t.contains("todo") || t.contains("fixme") || t.contains("hack") {
                Some((i + 1, l.trim().to_string()))
            } else {
                None
            }
        })
        .collect()
}

fn count_imports(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("use ") || t.starts_with("import ")
        })
        .count()
}

fn find_long_fns(lines: &[&str], fn_starts: &[usize], total_lines: usize) -> Vec<(usize, String)> {
    fn_starts
        .iter()
        .enumerate()
        .filter_map(|(idx, &start)| {
            let end = if idx + 1 < fn_starts.len() {
                fn_starts[idx + 1]
            } else {
                total_lines
            };
            let n = end - start;
            if n > 40 {
                let name = lines[start]
                    .trim()
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("?")
                    .to_string();
                Some((n, name))
            } else {
                None
            }
        })
        .collect()
}

fn compute_max_nesting(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") || t.starts_with("///") || t.starts_with("/*") {
                return None;
            }
            let indent = line.len() - line.trim_start().len();
            Some(if indent > 0 && line.starts_with('\t') {
                indent
            } else {
                indent / 4
            })
        })
        .max()
        .unwrap_or(0)
}

fn extract_high_complexity(lines: &[&str]) -> Vec<(usize, String)> {
    high_complexity_fns(lines)
        .iter()
        .map(|(ln, comp)| {
            let name = lines[*ln]
                .trim()
                .split_whitespace()
                .nth(1)
                .unwrap_or("?")
                .to_string();
            (*comp, name)
        })
        .collect()
}

// ── 工具函数（单文件分析） ────────────────────────────

/// 查找函数定义行号（先试 Rust fn，再试其他语言）
fn find_fn_positions(lines: &[&str]) -> Vec<usize> {
    let rust: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            (t.starts_with("fn ") || t.starts_with("pub fn ") || t.starts_with("pub(crate) fn "))
                && !t.starts_with("//")
                && !t.starts_with("///")
        })
        .map(|(i, _)| i)
        .collect();
    if !rust.is_empty() {
        return rust;
    }
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            (t.starts_with("def ") || t.starts_with("function ") || t.starts_with("func "))
                && !t.starts_with("//")
                && !t.starts_with("#")
                && !t.starts_with("--")
        })
        .map(|(i, _)| i)
        .collect()
}

/// 收集缺少 `///` 文档注释的公开函数
fn find_missing_doc_comments(lines: &[&str]) -> Vec<String> {
    let mut missing = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("pub fn ") || t.starts_with("pub(crate) fn ") {
            let has_doc = (1..=3).any(|off| i >= off && lines[i - off].trim().starts_with("///"));
            if !has_doc {
                missing.push(t.split_whitespace().nth(2).unwrap_or("?").to_string());
            }
        }
    }
    missing
}

/// 判断行是否为函数定义
fn is_fn_def(t: &str) -> bool {
    t.starts_with("fn ")
        || t.starts_with("pub fn ")
        || t.starts_with("def ")
        || t.starts_with("function ")
        || t.starts_with("func ")
}

/// 统计行中分支关键字出现次数
fn branch_kw_count(t: &str) -> usize {
    [
        "if ", "else if ", "for ", "while ", "match ", "case ", "catch ", "except",
    ]
    .iter()
    .filter(|kw| t.contains(*kw))
    .count()
}

/// 查找圈复杂度超过 10 的函数
fn high_complexity_fns(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut results = Vec::new();
    let mut in_fn = false;
    let mut start = 0;
    let mut comp = 0;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if is_fn_def(t) {
            if in_fn && comp > 10 {
                results.push((start, comp));
            }
            start = i;
            comp = 0;
            in_fn = true;
        } else if in_fn {
            comp += branch_kw_count(t);
        }
    }
    if in_fn && comp > 10 {
        results.push((start, comp));
    }
    results
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
        RuleResult::pass(
            "Scope 目录",
            format!("全部 {} 个 scope 存在", c.scopes.len()),
        )
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
        RuleResult::pass(
            "TODO/FIXME 密度",
            format!("{} 处, 密度 {:.1}‰", total_markers, density),
        )
    } else {
        let files_with_todos: Vec<String> = scanned
            .iter()
            .filter(|f| !f.todos.is_empty())
            .map(|f| format!("{} ({} 处)", f.path.display(), f.todos.len()))
            .collect();
        let mut details = vec![format!(
            "{} 处, 密度 {:.1}‰（阈值 5‰）",
            total_markers, density
        )];
        details.extend(files_with_todos);
        RuleResult::fail("TODO/FIXME 密度", details)
    }
}

/// 检查函数长度（阈值 40 行，超过 80 行大幅扣分）
fn check_fn_length(scanned: &[ScannedFile]) -> RuleResult {
    let all_long: Vec<String> = scanned
        .iter()
        .flat_map(|f| {
            f.long_fns
                .iter()
                .map(|(n, name)| {
                    let level = if *n > 80 { "大幅" } else { "" };
                    format!("{}: `{}` {} 行（{}超限）", f.path.display(), name, n, level)
                })
                .collect::<Vec<_>>()
        })
        .collect();
    if all_long.is_empty() {
        RuleResult::pass("函数长度", "全部函数 ≤ 40 行".to_string())
    } else {
        RuleResult::fail("函数长度", all_long)
    }
}

/// 检查 API 文档覆盖率（pub fn 缺少 `///` 注释）
fn check_api_docs(scanned: &[ScannedFile]) -> RuleResult {
    let all_missing: Vec<String> = scanned
        .iter()
        .flat_map(|f| {
            f.missing_docs
                .iter()
                .map(|name| format!("{}: `{}`", f.path.display(), name))
                .collect::<Vec<_>>()
        })
        .collect();
    if all_missing.is_empty() {
        RuleResult::pass("API 文档覆盖率", "全部 pub 函数有文档注释".to_string())
    } else {
        RuleResult::fail("API 文档覆盖率", all_missing)
    }
}

/// 检查结构复杂度（嵌套深度 > 4、圈复杂度 > 10）
fn check_complexity(scanned: &[ScannedFile]) -> RuleResult {
    let mut details = Vec::new();
    // 嵌套深度
    for f in scanned {
        if f.max_nesting > 4 {
            details.push(format!(
                "{}: 嵌套深度 {} 层",
                f.path.display(),
                f.max_nesting
            ));
        }
    }
    // 圈复杂度
    for f in scanned {
        for (comp, name) in &f.high_complexity {
            details.push(format!(
                "{}: `{}` 圈复杂度 {}",
                f.path.display(),
                name,
                comp
            ));
        }
    }
    if details.is_empty() {
        RuleResult::pass("结构复杂度", "嵌套 ≤ 4 层，圈复杂度 ≤ 10".to_string())
    } else {
        RuleResult::fail("结构复杂度", details)
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

/// 检查模块文档（//! 存在性）
fn check_mod_doc(scanned: &[ScannedFile]) -> RuleResult {
    let missing: Vec<String> = scanned
        .iter()
        .filter(|f| !f.has_mod_doc)
        .map(|f| f.path.display().to_string())
        .collect();
    if missing.is_empty() {
        RuleResult::pass("模块文档", "全部文件包含 //! 文档".to_string())
    } else {
        let total = scanned.len();
        let pct = (total - missing.len()) as f64 / total.max(1) as f64 * 100.0;
        let mut details = vec![format!(
            "{}/{} 文件缺少 //!（覆盖率 {:.0}%）",
            missing.len(),
            total,
            pct
        )];
        details.extend(missing);
        RuleResult::fail("模块文档", details)
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
        assert_eq!(
            result,
            Some(("cargo", vec!["check", "--quiet"], "cargo check"))
        );
    }

    #[test]
    fn test_lint_command_python() {
        let result = lint_command(&contract::Language::Python);
        assert_eq!(result, Some(("uv", vec!["check"], "uv check")));
    }

    #[test]
    fn test_lint_command_typescript() {
        let result = lint_command(&contract::Language::TypeScript);
        assert_eq!(
            result,
            Some(("npx", vec!["tsc", "--noEmit"], "tsc --noEmit"))
        );
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
    fn test_is_source_file_generated() {
        assert!(!is_source_file(Path::new("foo.freezed.dart")));
        assert!(!is_source_file(Path::new("foo.g.dart")));
        assert!(!is_source_file(Path::new("foo.pb.go")));
    }

    // ── find_fn_positions ────────────────────────────────────────

    #[test]
    fn test_find_fn_positions_rust() {
        let lines = vec!["fn a() {}", "fn b() {}"];
        assert_eq!(find_fn_positions(&lines), vec![0, 1]);
    }

    #[test]
    fn test_find_fn_positions_other_langs() {
        let lines = vec!["def foo(): pass", "def bar(): pass"];
        assert_eq!(find_fn_positions(&lines), vec![0, 1]);
    }

    #[test]
    fn test_find_fn_positions_skip_comment() {
        let lines = vec!["// fn commented() {}", "fn real() {}"];
        assert_eq!(find_fn_positions(&lines), vec![1]);
    }

    // ── find_missing_doc_comments ───────────────────────────────

    #[test]
    fn test_find_missing_doc_comments_all_documented() {
        let lines = vec!["/// docs", "pub fn foo() {}", "fn bar() {}"];
        assert!(find_missing_doc_comments(&lines).is_empty());
    }

    #[test]
    fn test_find_missing_doc_comments_missing() {
        let lines = vec!["pub fn foo() {}"];
        assert_eq!(find_missing_doc_comments(&lines), vec!["foo()"]);
    }

    // ── high_complexity_fns ─────────────────────────────────────

    #[test]
    fn test_high_complexity_fns_none() {
        let lines = vec!["fn simple() { let x = 1; }"];
        assert!(high_complexity_fns(&lines).is_empty());
    }

    #[test]
    fn test_high_complexity_fns_above_10() {
        let mut branches = String::new();
        for i in 0..12 {
            branches.push_str(&format!("if {} == 0 {{ }}\n", i));
        }
        let code = format!("fn complex() {{\n{}}}", branches);
        let lines: Vec<&str> = code.lines().collect();
        let result = high_complexity_fns(&lines);
        assert!(!result.is_empty());
    }

    // ── scan_one ─────────────────────────────────────────────────

    fn with_temp_file(content: &str) -> (tempfile::TempDir, ScannedFile) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        std::fs::write(&path, content).unwrap();
        let f = scan_one(&path).unwrap();
        (dir, f)
    }

    #[test]
    fn test_scan_one_basic() {
        let (_dir, f) = with_temp_file("use std::collections;\nfn main() {}\n// TODO: fix\n");
        assert_eq!(f.lines, 3);
        assert_eq!(f.imports, 1);
        assert_eq!(f.todos.len(), 1);
    }

    #[test]
    fn test_scan_one_fn_length() {
        let mut content = String::from("fn long() {\n");
        for _ in 0..45 {
            content.push_str("    let x = 1;\n");
        }
        content.push_str("}\n");
        let (_dir, f) = with_temp_file(&content);
        assert_eq!(f.long_fns.len(), 1);
    }

    #[test]
    fn test_scan_one_missing_docs() {
        let content = "pub fn foo() {}\npub fn bar() {}\n";
        let (_dir, f) = with_temp_file(content);
        assert_eq!(f.missing_docs.len(), 2);
    }

    #[test]
    fn test_scan_one_mod_doc() {
        let (_dir, f) = with_temp_file("//! module doc\nfn a() {}\n");
        assert!(f.has_mod_doc);

        let (_dir2, f2) = with_temp_file("fn a() {}\n");
        assert!(!f2.has_mod_doc);
    }

    #[test]
    fn test_scan_one_unreadable() {
        assert!(scan_one(Path::new("/nonexistent/file.rs")).is_none());
    }

    // ── check_* ──────────────────────────────────────────────────

    fn make_file(lines: usize, imports: usize, todos: usize) -> ScannedFile {
        ScannedFile {
            path: PathBuf::from("x.rs"),
            lines,
            todos: (0..todos).map(|i| (i, "TODO".into())).collect(),
            imports,
            long_fns: vec![],
            missing_docs: vec![],
            max_nesting: 0,
            high_complexity: vec![],
            has_mod_doc: true,
        }
    }

    #[test]
    fn test_check_todo_density_below() {
        assert!(check_todo_density(&[make_file(1000, 0, 0)]).passed);
    }

    #[test]
    fn test_check_todo_density_above() {
        assert!(!check_todo_density(&[make_file(1000, 0, 6)]).passed);
    }

    #[test]
    fn test_check_todo_density_empty() {
        assert!(check_todo_density(&[]).passed);
    }

    #[test]
    fn test_check_import_count_ok() {
        assert!(check_import_count(&[make_file(10, 5, 0)]).passed);
    }

    #[test]
    fn test_check_import_count_high() {
        assert!(!check_import_count(&[make_file(100, 35, 0)]).passed);
    }

    #[test]
    fn test_check_file_length_ok() {
        assert!(check_file_length(&[make_file(300, 0, 0)]).passed);
    }

    #[test]
    fn test_check_file_length_long() {
        assert!(!check_file_length(&[make_file(600, 0, 0)]).passed);
    }

    #[test]
    fn test_check_fn_length_ok() {
        let f = ScannedFile {
            long_fns: vec![],
            ..make_file(0, 0, 0)
        };
        assert!(check_fn_length(&[f]).passed);
    }

    #[test]
    fn test_check_fn_length_long() {
        let f = ScannedFile {
            long_fns: vec![(50, "foo".into())],
            ..make_file(0, 0, 0)
        };
        assert!(!check_fn_length(&[f]).passed);
    }

    #[test]
    fn test_check_api_docs_ok() {
        let f = ScannedFile {
            missing_docs: vec![],
            ..make_file(0, 0, 0)
        };
        assert!(check_api_docs(&[f]).passed);
    }

    #[test]
    fn test_check_api_docs_missing() {
        let f = ScannedFile {
            missing_docs: vec!["foo".into()],
            ..make_file(0, 0, 0)
        };
        assert!(!check_api_docs(&[f]).passed);
    }

    #[test]
    fn test_check_complexity_ok() {
        let f = ScannedFile {
            max_nesting: 3,
            high_complexity: vec![],
            ..make_file(0, 0, 0)
        };
        assert!(check_complexity(&[f]).passed);
    }

    #[test]
    fn test_check_complexity_deep_nesting() {
        let f = ScannedFile {
            max_nesting: 6,
            high_complexity: vec![],
            ..make_file(0, 0, 0)
        };
        assert!(!check_complexity(&[f]).passed);
    }

    #[test]
    fn test_check_complexity_high_cyclomatic() {
        let f = ScannedFile {
            max_nesting: 0,
            high_complexity: vec![(15, "foo".into())],
            ..make_file(0, 0, 0)
        };
        assert!(!check_complexity(&[f]).passed);
    }

    #[test]
    fn test_check_mod_doc_ok() {
        let f = ScannedFile {
            has_mod_doc: true,
            ..make_file(0, 0, 0)
        };
        assert!(check_mod_doc(&[f]).passed);
    }

    #[test]
    fn test_check_mod_doc_missing() {
        let f = ScannedFile {
            has_mod_doc: false,
            ..make_file(0, 0, 0)
        };
        assert!(!check_mod_doc(&[f]).passed);
    }

    // ── check_scope_dirs ─────────────────────────────────────────

    fn with_contract_scope(scope_dir: &str) -> (tempfile::TempDir, contract::Contract) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".quanttide/devops")).unwrap();
        std::fs::write(
            dir.path().join(".quanttide/devops/contract.yaml"),
            format!("stages:\nscopes:\n  core:\n    dir: {}\n", scope_dir),
        )
        .unwrap();
        let c = contract::load(dir.path());
        (dir, c)
    }

    #[test]
    fn test_check_scope_dirs_all_exist() {
        let (dir, c) = with_contract_scope("src");
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        assert!(check_scope_dirs(&c, dir.path()).passed);
    }

    #[test]
    fn test_check_scope_dirs_missing() {
        let (dir, c) = with_contract_scope("nonexistent");
        assert!(!check_scope_dirs(&c, dir.path()).passed);
    }
}
