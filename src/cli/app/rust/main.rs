use clap::{Parser, Subcommand};
use qtcloud_devops_cli::commands::editor::GitSubmoduleEditor;
use qtcloud_devops_cli::commands::{HealthIssue, SubmoduleEditor};
use qtcloud_devops_cli::model;
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

fn resolve_path(path: &PathBuf) -> Result<PathBuf, String> {
    std::fs::canonicalize(path)
        .map_err(|e| format!("无法解析路径 '{}': {}", path.display(), e))
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
            let result = run_code(dry_run, action);
            if let Err(e) = result {
                eprintln!("错误: {}", e);
                process::exit(1);
            }
        }
    }
}

fn run_code(dry_run: bool, action: CodeAction) -> Result<(), String> {
    match action {
        CodeAction::Status { path } => {
            let root = resolve_path(&path)?;
            let editor = GitSubmoduleEditor::new(root.clone());
            let state = model::RepoState::scan(&root)
                .map_err(|e| format!("{}", e))?;
            let issues = editor.status()
                .map_err(|e| format!("{}", e))?;

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
            Ok(())
        }

        CodeAction::Sync {
            name: Some(n),
            repo,
        } => {
            let root = resolve_path(&repo)?;
            if dry_run {
                println!("[预览] 同步子模块 '{}' 到父仓库", n);
                return Ok(());
            }
            let editor = GitSubmoduleEditor::new(root);
            editor.sync_to_parent(&n)
                .map_err(|e| format!("同步子模块 '{}' 失败: {}", n, e))
        }

        CodeAction::Sync { name: None, repo } => {
            let root = resolve_path(&repo)?;
            if dry_run {
                println!("[预览] 同步所有子模块到父仓库");
                return Ok(());
            }
            let editor = GitSubmoduleEditor::new(root);
            editor.sync_all_to_parent()
                .map_err(|e| format!("同步所有子模块失败: {}", e))
        }

        CodeAction::Retire { name, repo } => {
            let root = resolve_path(&repo)?;
            if dry_run {
                println!("[预览] 退役子模块 '{}'", name);
                return Ok(());
            }
            let editor = GitSubmoduleEditor::new(root);
            editor.retire_submodule(&name)
                .map_err(|e| format!("退役子模块 '{}' 失败: {}", name, e))
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ---- resolve_path ----

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

    // ---- print_issues ----

    #[test]
    fn test_print_issues_empty() {
        let issues = vec![];
        print_issues(&issues);
    }

    #[test]
    fn test_print_issues_non_empty() {
        use qtcloud_devops_cli::commands::HealthIssue;
        use qtcloud_devops_cli::model::SubmoduleStatus;
        let issues = vec![HealthIssue {
            submodule_name: "libs/foo".into(),
            status: SubmoduleStatus::Dirty,
            description: "有修改".into(),
            suggested_action: "提交".into(),
        }];
        print_issues(&issues);
    }

    // ---- print_aggregate ----

    #[test]
    fn test_print_aggregate_all_zeros() {
        let state = model::RepoState {
            root_path: PathBuf::from("/tmp"),
            submodules: vec![],
            total: 0,
            clean_count: 0,
            needs_attention: vec![],
        };
        print_aggregate(&state);
    }

    #[test]
    fn test_print_aggregate_with_variants() {
        use qtcloud_devops_cli::model::{CommitHash, Submodule, SubmoduleStatus};
        let submodules = vec![
            Submodule {
                name: "a".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::AheadOfParent,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "b".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::BehindRemote,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "c".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Detached,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "d".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Dirty,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "e".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Orphaned,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "f".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Uninitialized,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "g".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Clean,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
        ];
        let state = model::RepoState {
            root_path: PathBuf::from("/tmp"),
            submodules,
            total: 7,
            clean_count: 1,
            needs_attention: vec![
                "a".into(),
                "b".into(),
                "c".into(),
                "d".into(),
                "e".into(),
                "f".into(),
            ],
        };
        print_aggregate(&state);
    }
}
