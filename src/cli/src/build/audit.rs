use std::path::Path;

use crate::contract;

/// 构建审计：检查编译器配置、CI 工作流、依赖声明、编译器警告。
pub fn audit(repo_path: &Path) {
    let c = crate::contract::load(repo_path);
    println!("构建审计\n{}", "-".repeat(50));
    let mut passed = 0u32;
    let total = 5u32;

    if audit_compiler(repo_path) {
        passed += 1;
    }
    if audit_manifest(repo_path) {
        passed += 1;
    }
    if audit_ci_workflow(&c, repo_path) {
        passed += 1;
    }
    if audit_deps(repo_path) {
        passed += 1;
    }
    if audit_unused_vars(repo_path) {
        passed += 1;
    }

    print_audit_summary(passed, total);
}

fn audit_compiler(repo_path: &Path) -> bool {
    let lang = contract::detect_languages(repo_path)
        .into_iter()
        .next()
        .unwrap_or(contract::Language::Unknown(String::new()));
    if let Some((cmd, label)) = crate::build::check_command(&lang) {
        let ok = std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        println!("  {} 编译器: {}", if ok { "✅" } else { "❌" }, label);
        ok
    } else {
        println!("  ⚠ 编译器: 未知语言，跳过");
        true
    }
}

fn audit_manifest(repo_path: &Path) -> bool {
    let lang = contract::detect_languages(repo_path)
        .into_iter()
        .next()
        .unwrap_or(contract::Language::Unknown(String::new()));
    if let Some(f) = crate::build::check_manifest_file(&lang) {
        let exists = repo_path.join(f).exists();
        println!("  {} 清单文件: {}", if exists { "✅" } else { "❌" }, f);
        exists
    } else {
        println!("  ⚠ 清单文件: 未知语言，跳过");
        true
    }
}

fn audit_ci_workflow(c: &contract::Contract, repo_path: &Path) -> bool {
    if c.scopes.is_empty() {
        println!("  ⚠ CI 工作流: 无 scope，跳过");
        return true;
    }
    let all_ci = c.scopes.iter().all(|s| {
        let workflow = crate::build::resolve_workflow(&s.name, s.ci_workflow.as_deref());
        repo_path
            .join(".github/workflows")
            .join(format!("{}.yml", workflow))
            .exists()
            || repo_path
                .join(".github/workflows")
                .join(format!("{}.yaml", workflow))
                .exists()
    });
    println!(
        "  {} CI 工作流: {}",
        if all_ci { "✅" } else { "❌" },
        if all_ci {
            "全部 scope 已定义"
        } else {
            "部分 scope 缺少 CI 工作流"
        }
    );
    all_ci
}

fn audit_deps(repo_path: &Path) -> bool {
    let deps = crate::build::check_dependencies(repo_path);
    let ok = deps == "✅ crates.io" || deps == "—";
    println!("  {} 依赖来源: {}", if ok { "✅" } else { "❌" }, deps);
    ok
}

/// 审计未用变量：运行 cargo check 并解析编译器警告。
fn audit_unused_vars(repo_path: &Path) -> bool {
    let langs = crate::contract::detect_languages(repo_path);
    let is_rust = langs
        .iter()
        .any(|l| matches!(l, crate::contract::Language::Rust));
    if !is_rust {
        println!("  ⚠ 未用变量检测: 仅支持 Rust，跳过");
        return true;
    }

    let output = match std::process::Command::new("cargo")
        .args(["check", "--message-format=json", "--quiet"])
        .current_dir(repo_path)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            println!("  ❌ 未用变量检测: cargo check 执行失败: {}", e);
            return false;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut findings: Vec<(String, usize, String)> = Vec::new();
    for line in stdout.lines() {
        if let Some((file, line_no, msg)) = parse_unused_warning(line) {
            findings.push((file, line_no, msg));
        }
    }

    if findings.is_empty() {
        println!("  ✅ 未用变量检测: 未发现未用变量");
        true
    } else {
        for (file, line_no, msg) in &findings {
            println!("    ❌ {}:{} — {}", file, line_no, msg);
        }
        println!("  ❌ 未用变量检测: 发现 {} 处未用变量", findings.len());
        false
    }
}

/// 从 cargo check JSON 输出中解析未用变量/未用 mut 警告。
fn parse_unused_warning(line: &str) -> Option<(String, usize, String)> {
    if !line.starts_with('{') {
        return None;
    }
    let msg: serde_json::Value = serde_json::from_str(line).ok()?;
    if msg["reason"] != "compiler-message" || msg["message"]["level"] != "warning" {
        return None;
    }
    let code = msg["message"]["code"]["code"].as_str()?;
    if code != "unused_variables" && code != "unused_mut" {
        return None;
    }
    let span = msg["message"]["spans"].as_array()?.first()?;
    let file = span["file_name"].as_str()?.to_string();
    let line_no = span["line_start"].as_u64()? as usize;
    let message = msg["message"]["message"].as_str()?.to_string();
    Some((file, line_no, message))
}

fn print_audit_summary(passed: u32, total: u32) {
    println!("\n{}", "-".repeat(50));
    if passed == total {
        println!("  ✅ 全部 {} 项检查通过", total);
    } else {
        println!("  ⚠ {}/{} 项通过", passed, total);
    }
}
