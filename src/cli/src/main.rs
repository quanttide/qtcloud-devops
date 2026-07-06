use clap::{Parser, Subcommand};
use qtcloud_devops_cli::code::{self, StatusReport};
use qtcloud_devops_cli::release::PublishTarget;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "qtcloud-devops",
    about = "量潮DevOps实验室 — Git 子模块管理 & 发布管理",
    version
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
    /// 编辑 scope ROADMAP：读取原始格式 → 标准化 → 写回
    Edit {
        /// scope 名称
        scope: Option<String>,
    },
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
        /// 版本号。格式 `vX.Y.Z` 或 `scope/vX.Y.Z`。省略时自动检测。
        #[arg(short = 'v', long)]
        version: Option<String>,
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

    let result = match cli.command {
        Commands::Code { action } => run_code(action),
        Commands::Build { action } => match action {
            BuildAction::Status => {
                qtcloud_devops_cli::build::status(&repo_path());
                Ok(())
            }
            BuildAction::Clean => {
                qtcloud_devops_cli::build::clean(&repo_path());
                Ok(())
            }
            BuildAction::Audit => {
                qtcloud_devops_cli::build::audit(&repo_path());
                Ok(())
            }
        },
        Commands::Test { action } => match action {
            TestAction::Status => {
                let c = qtcloud_devops_cli::contract::load(&repo_path());
                qtcloud_devops_cli::test::status(&repo_path(), &c);
                Ok(())
            }
            TestAction::Clean => {
                qtcloud_devops_cli::test::clear_cache(&repo_path());
                Ok(())
            }
            TestAction::Audit { all, verbose } => {
                let c = qtcloud_devops_cli::contract::load(&repo_path());
                qtcloud_devops_cli::test::audit(&repo_path(), &c, all, verbose)
                    .map_err(|e| format!("{}", e))
            }
        },
        Commands::Release { action } => match action {
            ReleaseAction::Audit { version } => {
                let rp = repo_path();
                match qtcloud_devops_cli::release::audit(version.as_deref(), &rp) {
                    Ok(items) => {
                        println!("发布审计\n{}", "-".repeat(50));
                        let mut passed = 0u32;
                        for item in &items {
                            let icon = if item.passed { "✅" } else { "❌" };
                            println!("  {} {}", icon, item.name);
                            println!("        {}", item.detail);
                            if item.passed { passed += 1; }
                        }
                        println!("\n{}", "-".repeat(50));
                        let total = items.len() as u32;
                        if passed == total {
                            println!("  全部 {} 项检查通过", total);
                            Ok(())
                        } else {
                            Err(format!("{}/{} 项未通过", total - passed, total))
                        }
                    }
                    Err(e) => Err(format!("审计失败: {}", e)),
                }
            }
            ReleaseAction::Publish {
                version,
                yes,
                force,
                dry_run,
                registry,
            } => qtcloud_devops_cli::release::publish(
                version.as_deref(),
                &repo_path(),
                yes,
                force,
                dry_run,
                registry,
            )
            .map_err(|e| format!("{}", e)),
            ReleaseAction::Status => {
                qtcloud_devops_cli::release::status(&repo_path());
                Ok(())
            }
        },
        Commands::Plan { action } => match action {
            PlanAction::Status { scope } => qtcloud_devops_cli::plan::print_status(&repo_path(), scope.as_deref())
                .map_err(|e| format!("{}", e)),
            PlanAction::Clean { scope } => run_plan_clean(scope),
            PlanAction::Edit { scope } => run_plan_edit(scope),
            PlanAction::Audit => {
                let rp = repo_path();
                qtcloud_devops_cli::plan::plan_audit(&rp)
                    .map_err(|e| format!("{}", e))
            }
        },
        Commands::Contract { action } => match action {
            ContractAction::Status => {
                qtcloud_devops_cli::contract::status(&repo_path());
                Ok(())
            }
        },
        Commands::Status => {
            let rp = repo_path();
            qtcloud_devops_cli::contract::status(&rp);
            println!();
            qtcloud_devops_cli::source::status(&rp);
            println!();
            qtcloud_devops_cli::plan::print_status(&rp, None).ok();
            println!();
            if let Ok(report) = qtcloud_devops_cli::code::status(rp.clone(), false) {
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
        Commands::Audit => {
            let rp = repo_path();
            let c = qtcloud_devops_cli::contract::load(&rp);
            println!("概览审计\n{}", "-".repeat(50));
            qtcloud_devops_cli::build::audit(&rp);
            println!();
            qtcloud_devops_cli::test::audit(&rp, &c, true, false).ok();
            println!();
            qtcloud_devops_cli::release::audit(None, &rp).ok();
            Ok(())
        }
        Commands::Source { action } => match action {
            SourceAction::Status => {
                let rp = repo_path();
                qtcloud_devops_cli::source::status(&rp);
                Ok(())
            }
        },
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn run_code(action: CodeAction) -> Result<(), String> {
    match action {
        CodeAction::Status { path, offline } => run_code_status(path, offline),
        CodeAction::Audit { path } => {
            let root = resolve_path(&path)?;
            qtcloud_devops_cli::code::audit(&root);
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
    let roadmap_path = qtcloud_devops_cli::plan::resolve_roadmap_path(&repo_path, scope.as_deref());
    if !roadmap_path.exists() {
        println!("  未找到规划文件: {}", roadmap_path.display());
        return Ok(());
    }
    let removed = qtcloud_devops_cli::plan::clean_roadmap(&roadmap_path)
        .map_err(|e| format!("{}", e))?;
    if removed > 0 {
        println!(
            "  ✓ 已清理 {} 字节，文件: {}",
            removed,
            roadmap_path.display()
        );
        // 自动提交
        let rel = roadmap_path
            .strip_prefix(&repo_path)
            .unwrap_or(&roadmap_path)
            .to_str()
            .unwrap_or("ROADMAP.md");
        std::process::Command::new("git")
            .args(["add", rel])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| format!("git add 失败: {}", e))?;
        std::process::Command::new("git")
            .args(["commit", "-m", "chore: clean completed roadmap items"])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| format!("git commit 失败: {}", e))?;
        println!("  ✓ 已提交");
    } else {
        println!("  无已完成条目可清理");
    }
    Ok(())
}

fn run_plan_edit(scope: Option<String>) -> Result<(), String> {
    let repo_path = repo_path();
    let roadmap_path = qtcloud_devops_cli::plan::resolve_roadmap_path(&repo_path, scope.as_deref());
    if !roadmap_path.exists() {
        println!("  未找到规划文件: {}", roadmap_path.display());
        return Ok(());
    }
    let scope_label = scope.unwrap_or_else(|| "(auto)".to_string());
    let issues = qtcloud_devops_cli::plan::edit_roadmap(&roadmap_path, &scope_label)
        .map_err(|e| format!("{}", e))?;
    if issues.is_empty() {
        println!("  ✅ 格式无误");
    } else {
        println!("  📝 已转换 {} 处格式", issues.len());
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
