use std::path::{Path, PathBuf};

use crate::contract;

// ═══════════════════════════════════════════════════════════════════════
// audit — 代码质量审计
// ═══════════════════════════════════════════════════════════════════════

pub fn audit(repo_path: &Path) {
    let c = contract::load(repo_path);
    println!("代码审计\n{}", "-".repeat(50));
    let mut passed = 0u32;
    let total = 5u32;

    if audit_scope_dirs(&c, repo_path) {
        passed += 1;
    }

    let counts = collect_scope_markers(&c, repo_path);
    if audit_marker_density(&counts) {
        passed += 1;
    }
    if audit_imports(&counts) {
        passed += 1;
    }
    if audit_file_lengths(&counts) {
        passed += 1;
    }

    if audit_lint(repo_path) {
        passed += 1;
    }

    print_audit_summary(passed, total);
}

fn audit_scope_dirs(c: &contract::Contract, repo_path: &Path) -> bool {
    let all_ok = c.scopes.iter().all(|s| {
        let exists = repo_path.join(&s.dir).exists();
        if !exists {
            println!("  ❌ scope {}: 目录不存在 ({})", s.name, s.dir);
        }
        exists
    });
    if all_ok {
        println!("  ✅ Scope 目录: 全部 {} 个 scope 存在", c.scopes.len());
    }
    all_ok
}

fn collect_scope_markers(c: &contract::Contract, repo_path: &Path) -> MarkerCounts {
    let mut counts = MarkerCounts::default();
    for s in &c.scopes {
        count_markers(&repo_path.join(&s.dir), &mut counts);
    }
    counts
}

fn audit_marker_density(counts: &MarkerCounts) -> bool {
    if counts.lines == 0 {
        println!("  ⚠ TODO/FIXME: 无可扫描源码");
        return true;
    }
    let density = counts.markers as f64 / counts.lines as f64 * 1000.0;
    if density < 5.0 {
        println!(
            "  ✅ TODO/FIXME: {} 处, 密度 {:.1}‰",
            counts.markers, density
        );
        true
    } else {
        println!(
            "  ❌ TODO/FIXME: {} 处, 密度 {:.1}‰（阈值 5‰）",
            counts.markers, density
        );
        false
    }
}

fn audit_imports(counts: &MarkerCounts) -> bool {
    if counts.high_import_files.is_empty() {
        println!("  ✅ 导入数: 全部文件 ≤ 30 个 import");
        true
    } else {
        println!("  ❌ 导入数（≤30）:");
        for (path, count) in &counts.high_import_files {
            println!("     {} ({} 个 import)", path.display(), count);
        }
        false
    }
}

fn audit_file_lengths(counts: &MarkerCounts) -> bool {
    if counts.long_files.is_empty() {
        println!("  ✅ 文件长度: 全部文件 ≤ 500 行");
        true
    } else {
        println!("  ❌ 超长文件（阈值 500 行）:");
        for (path, line_count) in &counts.long_files {
            println!("     {} ({} 行)", path.display(), line_count);
        }
        false
    }
}

fn print_audit_summary(passed: u32, total: u32) {
    println!("\n{}", "-".repeat(50));
    if passed == total {
        println!("  ✅ 全部 {} 项检查通过", total);
    } else {
        println!("  ⚠ {}/{} 项通过", passed, total);
    }
}

fn audit_lint(repo_path: &Path) -> bool {
    match check_lint_for_langs(repo_path) {
        Some(true) => {
            println!("  ✅ 语法检查: 通过");
            true
        }
        Some(false) => {
            println!("  ❌ 语法检查: 存在错误");
            false
        }
        None => {
            println!("  ⚠ 语法检查: 跳过（不支持的语言）");
            true
        }
    }
}

#[derive(Default)]
struct MarkerCounts {
    markers: usize,
    lines: usize,
    imports: usize,
    long_files: Vec<(PathBuf, usize)>,
    high_import_files: Vec<(PathBuf, usize)>,
}

fn count_markers(dir: &Path, counts: &mut MarkerCounts) {
    const SRC_EXTENSIONS: &[&str] = &["rs", "py", "go", "ts", "tsx", "dart", "js", "jsx"];
    const GENERATED_SUFFIXES: &[&str] = &[
        ".freezed.dart",
        ".g.dart",
        ".grpc.dart",
        ".pb.dart",
        ".pb.go",
    ];
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
                count_markers(&path, counts);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map_or(false, |e| SRC_EXTENSIONS.contains(&e))
        {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if GENERATED_SUFFIXES.iter().any(|s| file_name.ends_with(s)) {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                let file_lines = content.lines().count();
                counts.lines += file_lines;
                counts.markers += content
                    .lines()
                    .filter(|l| {
                        let t = l.trim().to_lowercase();
                        t.contains("todo") || t.contains("fixme") || t.contains("hack")
                    })
                    .count();
                let import_count = content
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        t.starts_with("use ") || t.starts_with("import ")
                    })
                    .count();
                counts.imports += import_count;
                if import_count > 30 {
                    counts.high_import_files.push((path.clone(), import_count));
                }
                if file_lines > 500 {
                    counts.long_files.push((path, file_lines));
                }
            }
        }
    }
}

fn check_lint_for_langs(repo_path: &Path) -> Option<bool> {
    let langs = contract::detect_languages(repo_path);
    if langs.is_empty() {
        return None;
    }
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
            println!("     {}: {}", label, if ok { "✅" } else { "❌" });
        }
    }
    Some(all_ok)
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
    fn test_lint_command_unknown() {
        let result = lint_command(&contract::Language::Unknown("x".into()));
        assert_eq!(result, None);
    }

    #[test]
    fn test_lint_command_typescript() {
        let result = lint_command(&contract::Language::TypeScript);
        assert_eq!(
            result,
            Some(("npx", vec!["tsc", "--noEmit"], "tsc --noEmit"))
        );
    }

    // ── audit helper functions ──────────────────────────────────

    #[test]
    fn test_audit_marker_density_below_threshold() {
        let counts = MarkerCounts {
            markers: 2,
            lines: 1000,
            ..Default::default()
        };
        assert!(audit_marker_density(&counts));
    }

    #[test]
    fn test_audit_marker_density_above_threshold() {
        let counts = MarkerCounts {
            markers: 10,
            lines: 1000,
            ..Default::default()
        };
        assert!(!audit_marker_density(&counts));
    }

    #[test]
    fn test_audit_marker_density_zero_lines() {
        let counts = MarkerCounts::default();
        assert!(audit_marker_density(&counts));
    }

    #[test]
    fn test_audit_imports_all_ok() {
        let counts = MarkerCounts::default();
        assert!(audit_imports(&counts));
    }

    #[test]
    fn test_audit_imports_high() {
        let counts = MarkerCounts {
            high_import_files: vec![("x.rs".into(), 35)],
            ..Default::default()
        };
        assert!(!audit_imports(&counts));
    }

    #[test]
    fn test_audit_file_lengths_all_ok() {
        let counts = MarkerCounts::default();
        assert!(audit_file_lengths(&counts));
    }

    #[test]
    fn test_audit_file_lengths_long() {
        let counts = MarkerCounts {
            long_files: vec![("x.rs".into(), 600)],
            ..Default::default()
        };
        assert!(!audit_file_lengths(&counts));
    }
}
