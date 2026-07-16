use clap::{Parser, Subcommand};
use qtcloud_devops_cli::code::{self, StatusReport};
use qtcloud_devops_cli::release::PublishTarget;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "qtcloud-devops",
    about = "量潮DevOps实验室 — Git 子模块管理 & 发布管理",
    version,
    disable_help_subcommand(true)
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 组件同步管理命令集
    Code {
        #[command(subcommand)]
        action: CodeAction,
    },
    /// 构建状态管理
    Build {
        #[command(subcommand)]
        action: BuildAction,
    },
    /// 测试状态管理
    Test {
        #[command(subcommand)]
        action: TestAction,
    },
    /// 规划管理：ROADMAP.md 进度查看与维护
    Plan {
        #[command(subcommand)]
        action: PlanAction,
    },
    /// 查看契约状态
    Contract {
        #[command(subcommand)]
        action: ContractAction,
    },
    /// 发布管理命令集
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },
    /// 快速导览：按 stage 分组展示命令
    Help,
    /// 概览状态：聚合 build / test / release / contract / plan 状态
    Status,
    /// 概览审计：聚合 build / test / release 审计
    Audit,
    /// 环境诊断：检查外部依赖命令状态
    Source {
        #[command(subcommand)]
        action: SourceAction,
    },
}

#[derive(Subcommand)]
enum BuildAction {
    /// 查看构建状态
    Status,
    /// 清理构建产物（target/、dist/ 等）
    Clean,
    /// 构建审计：检查编译器配置、CI 工作流、依赖声明
    Audit,
}

#[derive(Subcommand)]
enum TestAction {
    /// 查看测试状态
    Status,
    /// 清理本地测试产物（覆盖率报告等）
    Clean,
    /// 质量审计：扫描测试覆盖率、错误变体覆盖、门禁达标情况
    Audit {
        /// 审计所有 scope（默认仅当前 scope）
        #[arg(long)]
        all: bool,
        /// 展示每个未覆盖函数详情
        #[arg(long, short = 'v')]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum PlanAction {
    /// 查看 scope 规划进度
    Status {
        /// scope 名称（省略时自动检测当前目录所属 scope）
        scope: Option<String>,
    },
    /// 删除 scope 已完成条目
    Clean {
        /// scope 名称
        scope: Option<String>,
    },
    /// 审计 ROADMAP 与 TODO 的关系：ROADMAP 是完整规划，TODO 是待办
    Audit,
    /// 修复 ROADMAP 和 TODO 的格式：标准化版本头、分类、checkbox
    Doctor {
        /// scope 名称
        scope: Option<String>,
    },
    /// 从审计 JSON 更新 TODO.md（从 stdin 读取 JSON）
    TodoFromAudit,
}

#[derive(Subcommand)]
enum ContractAction {
    /// 查看契约配置与状态
    Status,
}

#[derive(Subcommand)]
enum ReleaseAction {
    /// 发布预检审计：检查版本号、配置文件、CHANGELOG、工作区、标签冲突、远程可达性
    Audit {
        /// 版本号。格式 `vX.Y.Z` 或 `scope/vX.Y.Z`。省略时审计全部 scope。
        #[arg(short = 'v', long)]
        version: Option<String>,
        /// 仅审计指定 scope（不传时审计全部 scope）
        #[arg(long)]
        scope: Option<String>,
    },
    /// 发布版本：校验 CHANGELOG → 创建 tag → 推送到远端 → 创建 GitHub Release
    Publish {
        /// 版本号。格式 `vX.Y.Z` 或 `scope/vX.Y.Z`（如 `cli/v0.5.0`）。省略时自动检测。
        #[arg(short = 'v', long)]
        version: Option<String>,
        /// 跳过用户确认
        #[arg(long, short = 'y')]
        yes: bool,
        /// 强制重新发布：删除已存在的 tag 和 Release 后重新创建
        #[arg(long, short = 'f')]
        force: bool,
        /// 仅预览，不执行任何操作
        #[arg(long)]
        dry_run: bool,
        /// CI 发布目标（仅打印提示，不执行发布）
        #[arg(long, value_enum)]
        registry: Option<PublishTarget>,
    },
    /// 查看发布状态：版本号、标签、CHANGELOG、工作区状态
    Status,
}

#[derive(Subcommand)]
enum SourceAction {
    /// 检查系统依赖命令状态
    Status,
}

#[derive(Subcommand)]
enum CodeAction {
    /// 查看组件同步状态
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        offline: bool,
    },
    /// 审计代码质量：scope 目录、TODO/FIXME 密度、语法检查
    Audit {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// 以 JSON 格式输出（供 plan todo-from-audit 消费）
        #[arg(long)]
        json: bool,
    },
}

fn resolve_path(path: &PathBuf) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).map_err(|e| format!("无法解析路径 '{}': {}", path.display(), e))
}

fn print_report(report: &StatusReport) {
    println!("仓库: {}", report.root);
    println!("组件总数: {}", report.total);
    if report.pending > 0 {
        println!("待处理: {}", report.pending);
        for c in &report.components {
            if c.status != code::SyncStatus::Synced {
                let detail = match (c.ahead, c.behind) {
                    (a, 0) if a > 0 => format!(" (领先 {} 提交)", a),
                    (0, b) if b > 0 => format!(" (落后 {} 提交)", b),
                    (a, b) if a > 0 && b > 0 => format!(" (+{}/-{})", a, b),
                    _ => String::new(),
                };
                println!("  {:<20} {}{}", c.name, c.status.label(), detail);
            }
        }
    } else {
        println!("全部组件已同步");
    }
}

fn repo_path() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&cwd)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            PathBuf::from(std::str::from_utf8(&o.stdout).unwrap_or("").trim())
        }
        _ => cwd,
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = dispatch(cli) {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn dispatch(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Code { action } => run_code(action),
        Commands::Build { action } => run_build(action),
        Commands::Test { action } => run_test(action),
        Commands::Release { action } => run_release(action),
        Commands::Plan { action } => run_plan(action),
        Commands::Contract { action } => run_contract(action),
        Commands::Help => run_help(),
        Commands::Status => run_overall_status(),
        Commands::Audit => run_overall_audit(),
        Commands::Source { action } => run_source(action),
    }
}

fn run_build(action: BuildAction) -> Result<(), String> {
    match action {
        BuildAction::Status => { qtcloud_devops_cli::build::status(&repo_path()); Ok(()) }
        BuildAction::Clean => { qtcloud_devops_cli::build::clean(&repo_path()); Ok(()) }
        BuildAction::Audit => { qtcloud_devops_cli::build::audit(&repo_path()); Ok(()) }
    }
}

fn run_test(action: TestAction) -> Result<(), String> {
    match action {
        TestAction::Status => {
            let c = qtcloud_devops_cli::contract::load(&repo_path());
            qtcloud_devops_cli::test::status(&repo_path(), &c);
            Ok(())
        }
        TestAction::Clean => { qtcloud_devops_cli::test::clear_cache(&repo_path()); Ok(()) }
        TestAction::Audit { all, verbose } => {
            let c = qtcloud_devops_cli::contract::load(&repo_path());
            qtcloud_devops_cli::test::audit(&repo_path(), &c, all, verbose)
                .map_err(|e| format!("{}", e))
        }
    }
}

fn run_release(action: ReleaseAction) -> Result<(), String> {
    match action {
        ReleaseAction::Audit { version, scope } => run_release_audit(version, scope),
        ReleaseAction::Publish { version, yes, force, dry_run, registry } =>
            qtcloud_devops_cli::release::publish(
                version.as_deref(), &repo_path(), yes, force, dry_run, registry,
            ).map_err(|e| format!("{}", e)),
        ReleaseAction::Status => { qtcloud_devops_cli::release::status(&repo_path()); Ok(()) }
    }
}

fn run_release_audit(version: Option<String>, scope: Option<String>) -> Result<(), String> {
    let rp = repo_path();
    let results = if let Some(v) = version {
        qtcloud_devops_cli::release::audit(Some(&v), &rp)
            .map(|items| vec![("".to_string(), items)])
    } else {
        qtcloud_devops_cli::release::audit_all(&rp, scope.as_deref())
    };
    match results {
        Ok(all_items) => {
            let (all_passed, all_total) =
                all_items.iter().fold((0u32, 0u32), |(p, t), (_, items)| {
                    let sp = items.iter().filter(|i| i.passed).count() as u32;
                    (p + sp, t + items.len() as u32)
                });
            for (scope_name, items) in &all_items {
                if !scope_name.is_empty() {
                    println!("发布审计 — {}\n{}", scope_name, "-".repeat(50));
                } else {
                    println!("发布审计\n{}", "-".repeat(50));
                }
                let mut passed = 0u32;
                for item in items {
                    let icon = if item.passed { "✅" } else { "❌" };
                    println!("  {} {}", icon, item.name);
                    println!("        {}", item.detail);
                    if item.passed { passed += 1; }
                }
                let total = items.len() as u32;
                println!("{}\n  {}/{} 项通过\n", "-".repeat(50), passed, total);
            }
            let total = all_items.len() as u32;
            if all_passed == all_total {
                println!("  全部 {} 项检查通过 ({} scope)", all_total, total);
                Ok(())
            } else {
                Err(format!("{}/{} 项未通过 ({} scope)", all_total - all_passed, all_total, total))
            }
        }
        Err(e) => Err(format!("审计失败: {}", e)),
    }
}

fn run_plan(action: PlanAction) -> Result<(), String> {
    match action {
        PlanAction::Status { scope } =>
            qtcloud_devops_cli::plan::print_status(&repo_path(), scope.as_deref())
                .map_err(|e| format!("{}", e)),
        PlanAction::Clean { scope } => run_plan_clean(scope),
        PlanAction::Doctor { scope } => run_plan_doctor(scope),
        PlanAction::Audit => {
            qtcloud_devops_cli::plan::plan_audit(&repo_path()).map_err(|e| format!("{}", e))
        }
        PlanAction::TodoFromAudit => {
            let input = std::io::read_to_string(std::io::stdin().lock())
                .map_err(|e| format!("读取 stdin 失败: {}", e))?;
            qtcloud_devops_cli::plan::todo_from_audit(&repo_path(), &input)
                .map_err(|e| format!("{}", e))
        }
    }
}

fn run_contract(action: ContractAction) -> Result<(), String> {
    match action {
        ContractAction::Status => { qtcloud_devops_cli::contract::status(&repo_path()); Ok(()) }
    }
}

fn run_source(action: SourceAction) -> Result<(), String> {
    match action {
        SourceAction::Status => { qtcloud_devops_cli::source::status(&repo_path()); Ok(()) }
    }
}

fn run_help() -> Result<(), String> {
    println!("qtcloud-devops — 量潮DevOps命令行工具");
    println!();
    println!("Development lifecycle (stages):");
    println!("  build → test → release");
    println!();
    println!("  build status / clean / audit");
    println!("  test  status / clean / audit");
    println!("        release status / audit / publish");
    println!();
    println!("Cross-stage:");
    println!("  code status / audit");
    println!("  plan  status / clean / doctor / audit");
    println!("  contract status");
    println!("  source status");
    println!();
    println!("Overview:");
    println!("  status    聚合所有 status");
    println!("  audit     聚合所有 audit");
    println!();
    println!("Use `--help` on any command for detailed options.");
    Ok(())
}

fn run_overall_status() -> Result<(), String> {
    let rp = repo_path();
    qtcloud_devops_cli::contract::status(&rp);
    println!();
    qtcloud_devops_cli::source::status(&rp);
    println!();
    qtcloud_devops_cli::plan::print_status(&rp, None).ok();
    println!();
    if let Ok(report) = qtcloud_devops_cli::code::status(rp.clone(), true) {
        print_report(&report);
    }
    println!();
    qtcloud_devops_cli::build::status(&rp);
    println!();
    let c = qtcloud_devops_cli::contract::load(&rp);
    qtcloud_devops_cli::test::status(&rp, &c);
    println!();
    qtcloud_devops_cli::release::status(&rp);
    Ok(())
}

fn run_overall_audit() -> Result<(), String> {
    let rp = repo_path();
    let c = qtcloud_devops_cli::contract::load(&rp);
    println!("概览审计\n{}", "-".repeat(50));
    qtcloud_devops_cli::build::audit(&rp);
    println!();
    qtcloud_devops_cli::test::audit(&rp, &c, true, false).ok();
    println!();
    qtcloud_devops_cli::release::audit_all(&rp, None).ok();
    Ok(())
}

fn run_code(action: CodeAction) -> Result<(), String> {
    match action {
        CodeAction::Status { path, offline } => run_code_status(path, offline),
        CodeAction::Audit { path, json } => {
            let root = resolve_path(&path)?;
            if json {
                let output = qtcloud_devops_cli::code::audit_json(&root);
                println!("{}", output);
            } else {
                qtcloud_devops_cli::code::audit(&root);
            }
            Ok(())
        }
    }
}

fn run_code_status(path: PathBuf, offline: bool) -> Result<(), String> {
    let root = resolve_path(&path)?;
    let report = code::status(root, offline).map_err(|e| format!("{}", e))?;
    print_report(&report);
    Ok(())
}

fn run_plan_clean(scope: Option<String>) -> Result<(), String> {
    let repo_path = repo_path();
    let dir = qtcloud_devops_cli::plan::resolve_roadmap_dir(&repo_path, scope.as_deref());
    let cleaned_files: Vec<String> = ["ROADMAP.md", "TODO.md"]
        .iter()
        .filter_map(|name| {
            let path = dir.join(name);
            if !path.exists() {
                return None;
            }
            match qtcloud_devops_cli::plan::clean_done_items(&path) {
                Ok(removed) if removed > 0 => {
                    println!("  ✓ 已清理 {} 字节，文件: {}", removed, path.display());
                    let rel = path
                        .strip_prefix(&repo_path)
                        .unwrap_or(&path)
                        .to_str()
                        .unwrap_or(name)
                        .to_string();
                    Some(rel)
                }
                Ok(_) => {
                    None
                }
                Err(e) => {
                    eprintln!("  ✗ 清理失败 {}: {}", name, e);
                    None
                }
            }
        })
        .collect();

    if cleaned_files.is_empty() {
        println!("  无已完成条目可清理");
        return Ok(());
    }

    // 自动提交
    for rel in &cleaned_files {
        std::process::Command::new("git")
            .args(["add", rel])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| format!("git add 失败: {}", e))?;
    }
    let files_str = cleaned_files.join(", ");
    std::process::Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("chore: clean completed items from {}", files_str),
        ])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("git commit 失败: {}", e))?;
    println!("  ✓ 已提交 ({})", files_str);
    Ok(())
}

fn run_plan_doctor(scope: Option<String>) -> Result<(), String> {
    let repo_path = repo_path();
    let scope_label = scope.clone().unwrap_or_else(|| "(auto)".to_string());

    for file_name in &["ROADMAP.md", "TODO.md"] {
        let dir = qtcloud_devops_cli::plan::resolve_roadmap_dir(&repo_path, scope.as_deref());
        let path = dir.join(file_name);
        if !path.exists() {
            continue;
        }
        let issues = qtcloud_devops_cli::plan::doctor_file(&path, &scope_label)
            .map_err(|e| format!("{}", e))?;
        if issues.is_empty() {
            println!("  ✅ {} 格式无误", file_name);
        } else {
            println!("  📝 {}: 已转换 {} 处格式", file_name, issues.len());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path_valid() {
        let p = resolve_path(&std::env::current_dir().unwrap().join("Cargo.toml").into());
        assert!(p.is_ok());
        assert!(p.unwrap().is_absolute());
    }

    #[test]
    fn test_resolve_path_invalid() {
        let result = resolve_path(&PathBuf::from("/__nonexistent_path_12345__"));
        assert!(result.is_err());
    }
}
