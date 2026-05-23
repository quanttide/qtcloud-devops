use clap::{Parser, Subcommand};
use kse_core::commands::editor::GitSubmoduleEditor;
use kse_core::commands::{HealthIssue, SubmoduleEditor};
use kse_core::model;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "qtcloud-devops",
    about = "量潮DevOps实验室 — Git 子模块管理工具",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Git 子模块管理命令集
    Code {
        #[arg(global = true, long = "dry-run")]
        dry_run: bool,

        #[command(subcommand)]
        action: CodeAction,
    },
}

#[derive(Subcommand)]
enum CodeAction {
    /// 扫描并展示仓库所有子模块的状态
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// 同步子模块指针到父仓库
    Sync {
        /// 子模块名称（省略则同步全部）
        name: Option<String>,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// 退役子模块
    Retire {
        name: String,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
}

fn resolve_path(path: &PathBuf) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|e| {
        eprintln!("错误: 无法解析路径 '{}': {}", path.display(), e);
        process::exit(1);
    })
}

fn print_issues(issues: &[HealthIssue]) {
    if !issues.is_empty() {
        println!("\n需要关注的子模块:");
        for issue in issues {
            println!("  [{}] {}", issue.submodule_name, issue.description);
            println!("        建议: {}", issue.suggested_action);
        }
    }
}

fn print_aggregate(state: &model::RepoState) {
    if let Ok((_, agg)) = model::RepoState::scan_all(&state.root_path) {
        println!("\n聚合统计:");
        println!("  总数: {}", agg.total);
        println!("  ✅ Clean: {}", agg.clean);
        if agg.ahead_of_parent > 0 {
            println!("  ⬆ AheadOfParent: {}", agg.ahead_of_parent);
        }
        if agg.behind_remote > 0 {
            println!("  ⬇ BehindRemote: {}", agg.behind_remote);
        }
        if agg.detached > 0 {
            println!("  ⚠ Detached: {}", agg.detached);
        }
        if agg.dirty > 0 {
            println!("  🔴 Dirty: {}", agg.dirty);
        }
        if agg.orphaned > 0 {
            println!("  💀 Orphaned: {}", agg.orphaned);
        }
        if agg.uninitialized > 0 {
            println!("  ⚪ Uninitialized: {}", agg.uninitialized);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Code { dry_run, action } => {
            run_code(dry_run, action);
        }
    }
}

fn run_code(dry_run: bool, action: CodeAction) {
    match action {
        CodeAction::Status { path } => {
            let root = resolve_path(&path);
            let editor = GitSubmoduleEditor::new(root.clone());
            let state = model::RepoState::scan(&root).unwrap_or_else(|e| {
                eprintln!("错误: {}", e);
                process::exit(1);
            });
            let issues = editor.status().unwrap_or_else(|e| {
                eprintln!("错误: {}", e);
                process::exit(1);
            });

            println!("仓库: {}", state.root_path.display());
            println!("子模块总数: {}", state.total);
            println!("干净: {}", state.clean_count);
            if !state.needs_attention.is_empty() {
                println!("需要关注: {}", state.needs_attention.join(", "));
            }

            print_aggregate(&state);
            println!();

            if state.submodules.is_empty() && state.total == 0 {
                println!("  没有子模块");
            } else {
                println!(
                    "  {:<20} {:<15} {:<10} {:<8}",
                    "名称", "状态", "分支", "差异"
                );
                for sm in &state.submodules {
                    let diff = if sm.ahead_count > 0 && sm.behind_count > 0 {
                        format!("+{}/-{}", sm.ahead_count, sm.behind_count)
                    } else if sm.ahead_count > 0 {
                        format!("+{}", sm.ahead_count)
                    } else if sm.behind_count > 0 {
                        format!("-{}", sm.behind_count)
                    } else {
                        String::new()
                    };
                    println!(
                        "  {:<20} {:<15} {:<10} {:<8}",
                        sm.name,
                        format!("{:?}", sm.status),
                        sm.tracked_branch,
                        diff,
                    );
                }
            }
            print_issues(&issues);
        }

        CodeAction::Sync {
            name: Some(n),
            repo,
        } => {
            let root = resolve_path(&repo);
            if dry_run {
                println!("[预览] 同步子模块 '{}' 到父仓库", n);
                return;
            }
            let editor = GitSubmoduleEditor::new(root);
            exec(editor.sync_to_parent(&n));
        }

        CodeAction::Sync { name: None, repo } => {
            let root = resolve_path(&repo);
            if dry_run {
                println!("[预览] 同步所有子模块到父仓库");
                return;
            }
            let editor = GitSubmoduleEditor::new(root);
            exec(editor.sync_all_to_parent());
        }

        CodeAction::Retire { name, repo } => {
            let root = resolve_path(&repo);
            if dry_run {
                println!("[预览] 退役子模块 '{}'", name);
                return;
            }
            let editor = GitSubmoduleEditor::new(root);
            exec(editor.retire_submodule(&name));
        }

    }
}

fn exec<T>(result: Result<T, Box<dyn std::error::Error>>) {
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("错误: {}", e);
            process::exit(1);
        }
    }
}
