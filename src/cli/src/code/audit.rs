use std::path::Path;

use crate::contract;

pub fn audit(repo_path: &Path) {
    let c = contract::load(repo_path);
    println!("代码审计\n{}", "-".repeat(50));
    let mut passed = 0u32;
    let total = 3u32;

    // ── 1. Scope 目录结构 ─────────────────────────────────────────
    let all_ok = c.scopes.iter().all(|s| {
        let dir = repo_path.join(&s.dir);
        let exists = dir.exists();
        if !exists {
            println!("  ❌ scope {}: 目录不存在 ({})", s.name, s.dir);
        }
        exists
    });
    if all_ok {
        println!("  ✅ Scope 目录: 全部 {} 个 scope 存在", c.scopes.len());
        passed += 1;
    }

    // ── 2. TODO/FIXME 密度 ────────────────────────────────────────
    let mut total_markers = 0usize;
    let mut total_lines = 0usize;
    for s in &c.scopes {
        let dir = repo_path.join(&s.dir);
        count_markers(&dir, &mut total_markers, &mut total_lines);
    }
    if total_lines > 0 {
        let density = total_markers as f64 / total_lines as f64 * 1000.0;
        if density < 5.0 {
            println!("  ✅ TODO/FIXME: {} 处, 密度 {:.1}‰", total_markers, density);
            passed += 1;
        } else {
            println!("  ❌ TODO/FIXME: {} 处, 密度 {:.1}‰（阈值 5‰）", total_markers, density);
        }
    } else {
        println!("  ⚠ TODO/FIXME: 无可扫描源码");
        passed += 1;
    }

    // ── 3. 语法检查 ───────────────────────────────────────────────
    match check_lint_for_langs(repo_path) {
        Some(true) => { println!("  ✅ 语法检查: 通过"); passed += 1; }
        Some(false) => println!("  ❌ 语法检查: 存在错误"),
        None => { println!("  ⚠ 语法检查: 跳过（不支持的语言）"); passed += 1; }
    }

    println!("\n{}", "-".repeat(50));
    if passed == total {
        println!("  ✅ 全部 {} 项检查通过", total);
    } else {
        println!("  ⚠ {}/{} 项通过", passed, total);
    }
}

/// 递归扫描目录下的 .rs 文件，统计 TODO/FIXME 标记数。
fn count_markers(dir: &Path, markers: &mut usize, lines: &mut usize) {
    if !dir.exists() { return; }
    let entries = match std::fs::read_dir(dir) { Ok(e) => e, Err(_) => return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name != "target" && !name.starts_with('.') && name != "node_modules" {
                count_markers(&path, markers, lines);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                *lines += content.lines().count();
                *markers += content.lines().filter(|l| {
                    let t = l.trim().to_lowercase();
                    t.contains("todo") || t.contains("fixme") || t.contains("hack")
                }).count();
            }
        }
    }
}

/// 尝试运行语言对应的 lint 命令。
fn check_lint_for_langs(repo_path: &Path) -> Option<bool> {
    let langs = contract::detect_languages(repo_path);
    if langs.is_empty() { return None; }
    let mut all_ok = true;
    for lang in &langs {
        let (cmd, args, label) = lint_command(lang)?;
        let ok = std::process::Command::new(cmd)
            .args(&args)
            .current_dir(repo_path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok { all_ok = false; }
        println!("     {}: {}", label, if ok { "✅" } else { "❌" });
    }
    Some(all_ok)
}

fn lint_command(lang: &contract::Language) -> Option<(&'static str, Vec<&'static str>, &'static str)> {
    match lang {
        contract::Language::Rust => Some(("cargo", vec!["check", "--quiet"], "cargo check")),
        contract::Language::Python => Some(("uv", vec!["check"], "uv check")),
        contract::Language::TypeScript => Some(("npx", vec!["tsc", "--noEmit"], "tsc --noEmit")),
        _ => None,
    }
}
