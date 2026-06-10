use clap::{Parser, Subcommand};
use qtcloud_devops_cli::code::{self, StatusReport};
use qtcloud_devops_cli::release::Registry;
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
    /// 发布管理命令集
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },
}

#[derive(Subcommand)]
enum ReleaseAction {
    /// 发布版本：校验 CHANGELOG → 创建 tag → 推送到远端 → 创建 GitHub Release
    Publish {
        /// 版本号。格式 `vX.Y.Z` 或 `scope/vX.Y.Z`（如 `cli/v0.5.0`）
        #[arg(short = 'v', long)]
        version: String,
        /// 跳过用户确认
        #[arg(long, short = 'y')]
        yes: bool,
        /// CI 发布目标（仅打印提示，不执行发布）
        #[arg(long, value_enum)]
        registry: Option<Registry>,
    },
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
    /// 同步组件到远端
    Sync {
        /// 组件名称（省略则同步全部）
        name: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
}

fn resolve_path(path: &PathBuf) -> Result<PathBuf, String> {
    std::fs::canonicalize(path)
        .map_err(|e| format!("无法解析路径 '{}': {}", path.display(), e))
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
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Code { action } => run_code(action),
        Commands::Release { action } => match action {
            ReleaseAction::Publish { version, yes, registry } => {
                qtcloud_devops_cli::release::publish(&version, &repo_path(), yes, registry)
                    .map_err(|e| format!("{}", e))
            }
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn run_code(action: CodeAction) -> Result<(), String> {
    match action {
        CodeAction::Status { path, offline } => run_code_status(path, offline),
        CodeAction::Sync { name: Some(n), dry_run, repo } => {
            run_code_sync_one(&n, dry_run, repo)
        }
        CodeAction::Sync { name: None, dry_run, repo } => {
            run_code_sync_all(dry_run, repo)
        }
    }
}

fn run_code_status(path: PathBuf, offline: bool) -> Result<(), String> {
    let root = resolve_path(&path)?;
    let report = code::status(root, offline)?;
    print_report(&report);
    Ok(())
}

fn run_code_sync_one(name: &str, dry_run: bool, repo: PathBuf) -> Result<(), String> {
    let root = resolve_path(&repo)?;
    if dry_run {
        println!("[预览] 同步组件 '{}'", name);
        return Ok(());
    }
    code::sync(root, name)
}

fn run_code_sync_all(dry_run: bool, repo: PathBuf) -> Result<(), String> {
    let root = resolve_path(&repo)?;
    if dry_run {
        println!("[预览] 同步所有组件");
        return Ok(());
    }
    code::sync_all(root)
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
