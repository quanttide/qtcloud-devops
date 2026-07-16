use std::path::Path;

use crate::contract;

/// 构建审计：检查编译器配置、CI 工作流、依赖声明。
pub fn audit(repo_path: &Path) {
    let c = crate::contract::load(repo_path);
    println!("构建审计\n{}", "-".repeat(50));
    let mut passed = 0u32;
    let total = 4u32;

    if audit_compiler(repo_path) { passed += 1; }
    if audit_manifest(repo_path) { passed += 1; }
    if audit_ci_workflow(&c, repo_path) { passed += 1; }
    if audit_deps(repo_path) { passed += 1; }

    print_audit_summary(passed, total);
}

fn audit_compiler(repo_path: &Path) -> bool {
    let lang = contract::detect_languages(repo_path).into_iter().next()
        .unwrap_or(contract::Language::Unknown(String::new()));
    if let Some((cmd, label)) = crate::build::check_command(&lang) {
        let ok = std::process::Command::new(cmd).arg("--version").output()
            .map(|o| o.status.success()).unwrap_or(false);
        println!("  {} 编译器: {}", if ok { "✅" } else { "❌" }, label);
        ok
    } else {
        println!("  ⚠ 编译器: 未知语言，跳过");
        true
    }
}

fn audit_manifest(repo_path: &Path) -> bool {
    let lang = contract::detect_languages(repo_path).into_iter().next()
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
        repo_path.join(".github/workflows").join(format!("{}.yml", workflow)).exists()
            || repo_path.join(".github/workflows").join(format!("{}.yaml", workflow)).exists()
    });
    println!("  {} CI 工作流: {}", if all_ci { "✅" } else { "❌" },
        if all_ci { "全部 scope 已定义" } else { "部分 scope 缺少 CI 工作流" });
    all_ci
}

fn audit_deps(repo_path: &Path) -> bool {
    let deps = crate::build::check_dependencies(repo_path);
    let ok = deps == "✅ crates.io" || deps == "—";
    println!("  {} 依赖来源: {}", if ok { "✅" } else { "❌" }, deps);
    ok
}

fn print_audit_summary(passed: u32, total: u32) {
    println!("\n{}", "-".repeat(50));
    if passed == total {
        println!("  ✅ 全部 {} 项检查通过", total);
    } else {
        println!("  ⚠ {}/{} 项通过", passed, total);
    }
}
