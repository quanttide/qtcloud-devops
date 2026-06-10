use clap::{Parser, Subcommand};
use qtcloud_devops_cli::commands::code::GitSubmoduleEditor;
use qtcloud_devops_cli::commands::release::Registry;
use qtcloud_devops_cli::commands::{HealthIssue, SubmoduleEditor};
use qtcloud_devops_cli::model::code;
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
    /// Git 子模块管理命令集
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
    /// 预发布版本（rc），创建 tag + GitHub Release
    Stage {
        #[arg(short = 'v', long)]
        version: String,
    },
    /// 正式发布，创建 tag + GitHub Release
    Publish {
        #[arg(short = 'v', long)]
        version: String,
        #[arg(long, short = 'y')]
        yes: bool,
        #[arg(long, value_enum)]
        registry: Option<Registry>,
    },
}

#[derive(Subcommand)]
enum CodeAction {
    /// 扫描并展示仓库所有子模块的状态
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        offline: bool,
    },
    /// 同步子模块指针到父仓库
    Sync {
        /// 子模块名称（省略则同步全部）
        name: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// 退役子模块
    Retire {
        name: String,
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

fn print_issues(issues: &[HealthIssue]) {
    if !issues.is_empty() {
        println!("\n需要关注的子模块:");
        for issue in issues {
            println!("  [{}] {}", issue.submodule_name, issue.description);
            println!("        建议: {}", issue.suggested_action);
        }
    }
}

fn print_aggregate(state: &code::RepoState) {
    if let Ok((_, agg)) = code::RepoState::scan_all(&state.root_path) {
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

fn repo_path() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Code { action } => run_code(action),
        Commands::Release { action } => match action {
            ReleaseAction::Stage { version } => {
                qtcloud_devops_cli::commands::release::stage(&version, &repo_path())
                    .map_err(|e| format!("{}", e))
            }
            ReleaseAction::Publish { version, yes, registry } => {
                qtcloud_devops_cli::commands::release::publish(&version, &repo_path(), yes, registry)
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
        CodeAction::Retire { name, dry_run, repo } => {
            run_code_retire(&name, dry_run, repo)
        }
    }
}

fn run_code_status(path: PathBuf, offline: bool) -> Result<(), String> {
    let root = resolve_path(&path)?;
    let mut editor = GitSubmoduleEditor::new(root.clone());
    editor.set_offline(offline);
    let state = code::RepoState::scan(&root).map_err(|e| format!("{}", e))?;
    let issues = editor.status().map_err(|e| format!("{}", e))?;

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
        println!("  {:<20} {:<15} {:<10} {:<8}", "名称", "状态", "分支", "差异");
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
            println!("  {:<20} {:<15} {:<10} {:<8}", sm.name, format!("{:?}", sm.status), sm.tracked_branch, diff);
        }
    }
    print_issues(&issues);
    Ok(())
}

fn run_code_sync_one(name: &str, dry_run: bool, repo: PathBuf) -> Result<(), String> {
    let root = resolve_path(&repo)?;
    if dry_run {
        println!("[预览] 同步子模块 '{}' 到父仓库", name);
        return Ok(());
    }
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_to_parent(name).map_err(|e| format!("同步子模块 '{}' 失败: {}", name, e))
}

fn run_code_sync_all(dry_run: bool, repo: PathBuf) -> Result<(), String> {
    let root = resolve_path(&repo)?;
    if dry_run {
        println!("[预览] 同步所有子模块到父仓库");
        return Ok(());
    }
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_all_to_parent().map_err(|e| format!("同步所有子模块失败: {}", e))
}

fn run_code_retire(name: &str, dry_run: bool, repo: PathBuf) -> Result<(), String> {
    let root = resolve_path(&repo)?;
    if dry_run {
        println!("[预览] 退役子模块 '{}'", name);
        return Ok(());
    }
    let editor = GitSubmoduleEditor::new(root);
    editor.retire_submodule(name).map_err(|e| format!("退役子模块 '{}' 失败: {}", name, e))
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
        use qtcloud_devops_cli::model::code::SubmoduleStatus;
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
        let state = code::RepoState {
            root_path: PathBuf::from("/tmp"),
            submodules: vec![],
            total: 0,
            clean_count: 0,
            needs_attention: vec![],
            parent_dirty: false,
        };
        print_aggregate(&state);
    }

    #[test]
    fn test_print_aggregate_with_variants() {
        use qtcloud_devops_cli::model::code::{Submodule, SubmoduleStatus};

        fn sm(name: &str, status: SubmoduleStatus) -> Submodule {
            use qtcloud_devops_cli::model::code::CommitHash;
            Submodule {
                name: name.into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            }
        }

        let submodules = vec![
            sm("a", SubmoduleStatus::AheadOfParent),
            sm("b", SubmoduleStatus::BehindRemote),
            sm("c", SubmoduleStatus::Detached),
            sm("d", SubmoduleStatus::Dirty),
            sm("e", SubmoduleStatus::Orphaned),
            sm("f", SubmoduleStatus::Uninitialized),
            sm("g", SubmoduleStatus::Clean),
        ];
        let state = code::RepoState {
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
            parent_dirty: false,
        };
        print_aggregate(&state);
    }
}
