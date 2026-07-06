use std::path::{Path, PathBuf};

use crate::contract;
use crate::git::{RepoState, SubmoduleStatus};

// ═══════════════════════════════════════════════════════════════════════
// 模型
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Synced,
    PendingPush,
    PendingPull,
    Conflict,
}

impl SyncStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Synced => "已同步",
            Self::PendingPush => "待推送",
            Self::PendingPull => "待拉取",
            Self::Conflict => "冲突",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStatus {
    pub name: String,
    pub status: SyncStatus,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub root: String,
    pub components: Vec<ComponentStatus>,
    pub total: usize,
    pub synced: usize,
    pub pending: usize,
}

// ═══════════════════════════════════════════════════════════════════════
// status — 子模块同步状态
// ═══════════════════════════════════════════════════════════════════════

fn map_status(s: &SubmoduleStatus) -> SyncStatus {
    match s {
        SubmoduleStatus::Clean => SyncStatus::Synced,
        SubmoduleStatus::AheadOfParent => SyncStatus::PendingPush,
        SubmoduleStatus::BehindRemote => SyncStatus::PendingPull,
        _ => SyncStatus::Conflict,
    }
}

pub fn status(root: PathBuf, offline: bool) -> Result<StatusReport, Box<dyn std::error::Error>> {
    let state = if offline {
        RepoState::scan_offline(&root)
    } else {
        RepoState::scan(&root)
    }?;
    let mut components = Vec::with_capacity(state.submodules.len());
    for sm in &state.submodules {
        components.push(ComponentStatus {
            name: sm.name.clone(),
            status: map_status(&sm.status),
            ahead: sm.ahead_count,
            behind: sm.behind_count,
        });
    }
    let total = components.len();
    let synced = components
        .iter()
        .filter(|c| c.status == SyncStatus::Synced)
        .count();
    Ok(StatusReport {
        root: state.root_path.to_string_lossy().to_string(),
        components,
        total,
        synced,
        pending: total - synced,
    })
}

// ═══════════════════════════════════════════════════════════════════════
// audit — 代码质量审计
// ═══════════════════════════════════════════════════════════════════════

pub fn audit(repo_path: &Path) {
    let c = contract::load(repo_path);
    println!("代码审计\n{}", "-".repeat(50));
    let mut passed = 0u32;
    let total = 6u32;

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

    let mut total_markers = 0usize;
    let mut total_lines = 0usize;
    let mut total_unwraps = 0usize;
    let mut total_imports = 0usize;
    let mut long_files = Vec::new();
    let mut high_import_files = Vec::new();
    for s in &c.scopes {
        count_markers(
            &repo_path.join(&s.dir),
            &mut total_markers,
            &mut total_lines,
            &mut total_unwraps,
            &mut total_imports,
            &mut long_files,
            &mut high_import_files,
        );
    }
    if total_lines > 0 {
        let density = total_markers as f64 / total_lines as f64 * 1000.0;
        if density < 5.0 {
            println!(
                "  ✅ TODO/FIXME: {} 处, 密度 {:.1}‰",
                total_markers, density
            );
            passed += 1;
        } else {
            println!(
                "  ❌ TODO/FIXME: {} 处, 密度 {:.1}‰（阈值 5‰）",
                total_markers, density
            );
        }
    } else {
        println!("  ⚠ TODO/FIXME: 无可扫描源码");
        passed += 1;
    }

    if total_lines > 0 {
        let unwrap_density = total_unwraps as f64 / total_lines as f64 * 1000.0;
        if unwrap_density < 10.0 {
            println!(
                "  ✅ unwrap/expect: {} 处, 密度 {:.1}‰",
                total_unwraps, unwrap_density
            );
            passed += 1;
        } else {
            println!(
                "  ❌ unwrap/expect: {} 处, 密度 {:.1}‰（阈值 10‰）",
                total_unwraps, unwrap_density
            );
        }
    } else {
        println!("  ⚠ unwrap/expect: 无可扫描源码");
        passed += 1;
    }

    if high_import_files.is_empty() {
        println!("  ✅ 导入数: 全部文件 ≤ 30 个 import");
        passed += 1;
    } else {
        println!("  ❌ 导入数（≤30）:");
        for (path, count) in &high_import_files {
            println!("     {} ({} 个 import)", path.display(), count);
        }
    }

    if long_files.is_empty() {
        println!("  ✅ 文件长度: 全部文件 ≤ 500 行");
        passed += 1;
    } else {
        println!("  ❌ 超长文件（阈值 500 行）:");
        for (path, line_count) in &long_files {
            println!("     {} ({} 行)", path.display(), line_count);
        }
    }

    match check_lint_for_langs(repo_path) {
        Some(true) => {
            println!("  ✅ 语法检查: 通过");
            passed += 1;
        }
        Some(false) => println!("  ❌ 语法检查: 存在错误"),
        None => {
            println!("  ⚠ 语法检查: 跳过（不支持的语言）");
            passed += 1;
        }
    }

    println!("\n{}", "-".repeat(50));
    if passed == total {
        println!("  ✅ 全部 {} 项检查通过", total);
    } else {
        println!("  ⚠ {}/{} 项通过", passed, total);
    }
}

fn count_markers(
    dir: &Path,
    markers: &mut usize,
    lines: &mut usize,
    unwraps: &mut usize,
    imports: &mut usize,
    long_files: &mut Vec<(PathBuf, usize)>,
    high_import_files: &mut Vec<(PathBuf, usize)>,
) {
    const SRC_EXTENSIONS: &[&str] = &["rs", "py", "go", "ts", "tsx", "dart", "js", "jsx"];
    // ponytail: 跳过 codegen 文件（freezed/gRPC/protobuf/json-serializable 等），它们过长或含大量 unwrap 是常态
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
                count_markers(
                    &path,
                    markers,
                    lines,
                    unwraps,
                    imports,
                    long_files,
                    high_import_files,
                );
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
                *lines += file_lines;
                *markers += content
                    .lines()
                    .filter(|l| {
                        let t = l.trim().to_lowercase();
                        t.contains("todo") || t.contains("fixme") || t.contains("hack")
                    })
                    .count();
                for l in content.lines() {
                    *unwraps += l.matches(".unwrap()").count() + l.matches(".expect(").count();
                }
                let import_count = content
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        t.starts_with("use ") || t.starts_with("import ")
                    })
                    .count();
                *imports += import_count;
                if import_count > 30 {
                    high_import_files.push((path.clone(), import_count));
                }
                if file_lines > 500 {
                    long_files.push((path, file_lines));
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

    // ── SyncStatus ──────────────────────────────────────────────

    #[test]
    fn test_sync_status_labels() {
        assert_eq!(SyncStatus::Synced.label(), "已同步");
        assert_eq!(SyncStatus::PendingPush.label(), "待推送");
        assert_eq!(SyncStatus::PendingPull.label(), "待拉取");
        assert_eq!(SyncStatus::Conflict.label(), "冲突");
    }

    #[test]
    fn test_sync_status_clone_eq() {
        assert_eq!(SyncStatus::Synced, SyncStatus::Synced);
        assert_ne!(SyncStatus::Synced, SyncStatus::PendingPush);
    }

    #[test]
    fn test_component_status_builder() {
        let c = ComponentStatus {
            name: "libs/foo".into(),
            status: SyncStatus::PendingPush,
            ahead: 3,
            behind: 0,
        };
        assert_eq!(c.name, "libs/foo");
        assert_eq!(c.ahead, 3);
    }

    #[test]
    fn test_status_report_counts() {
        let report = StatusReport {
            root: "/tmp".into(),
            components: vec![
                ComponentStatus {
                    name: "a".into(),
                    status: SyncStatus::Synced,
                    ahead: 0,
                    behind: 0,
                },
                ComponentStatus {
                    name: "b".into(),
                    status: SyncStatus::PendingPush,
                    ahead: 1,
                    behind: 0,
                },
                ComponentStatus {
                    name: "c".into(),
                    status: SyncStatus::PendingPull,
                    ahead: 0,
                    behind: 2,
                },
                ComponentStatus {
                    name: "d".into(),
                    status: SyncStatus::Conflict,
                    ahead: 0,
                    behind: 0,
                },
            ],
            total: 4,
            synced: 1,
            pending: 3,
        };
        assert_eq!(report.total, 4);
        assert_eq!(report.synced, 1);
        assert_eq!(report.pending, 3);
    }

    // ── map_status ──────────────────────────────────────────────

    #[test]
    fn test_map_clean() {
        assert_eq!(map_status(&SubmoduleStatus::Clean), SyncStatus::Synced);
    }
    #[test]
    fn test_map_ahead() {
        assert_eq!(
            map_status(&SubmoduleStatus::AheadOfParent),
            SyncStatus::PendingPush
        );
    }
    #[test]
    fn test_map_behind() {
        assert_eq!(
            map_status(&SubmoduleStatus::BehindRemote),
            SyncStatus::PendingPull
        );
    }
    #[test]
    fn test_map_detached() {
        assert_eq!(map_status(&SubmoduleStatus::Detached), SyncStatus::Conflict);
    }
    #[test]
    fn test_map_dirty() {
        assert_eq!(map_status(&SubmoduleStatus::Dirty), SyncStatus::Conflict);
    }
    #[test]
    fn test_map_orphaned() {
        assert_eq!(map_status(&SubmoduleStatus::Orphaned), SyncStatus::Conflict);
    }
    #[test]
    fn test_map_uninitialized() {
        assert_eq!(
            map_status(&SubmoduleStatus::Uninitialized),
            SyncStatus::Conflict
        );
    }

    // ── status (integration) ──────────────────────────────────

    fn git_init(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        cfg.set_str("user.name", "t").unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        let repo = git2::Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("f")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().and_then(|h| h.peel_to_commit()).ok();
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
            .unwrap();
    }

    #[test]
    fn test_status_non_git_dir() {
        assert!(status(tempfile::tempdir().unwrap().path().to_path_buf(), false).is_err());
    }

    #[test]
    fn test_status_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        let report = status(d.path().to_path_buf(), false).unwrap();
        assert_eq!(report.total, 0);
        assert_eq!(report.synced, 0);
        assert_eq!(report.pending, 0);
    }

    #[test]
    fn test_status_with_synced_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("parent");
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        git_init(&sub);
        git_commit(&sub, "init sub");
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init parent");
        std::process::Command::new("git")
            .args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent)
            .output()
            .unwrap();
        let report = status(parent, false).unwrap();
        assert_eq!(report.total, 1);
        assert_eq!(report.components[0].status, SyncStatus::Synced);
    }

    #[test]
    fn test_status_offline_flag() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        assert!(status(d.path().to_path_buf(), true).is_ok());
    }
}
