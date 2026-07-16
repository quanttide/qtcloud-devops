use std::path::Path;

/// 清理构建产物。
pub fn clean(repo_path: &Path) {
    let targets = &[
        repo_path.join("target"),
        repo_path.join("dist"),
        repo_path.join("node_modules"),
        repo_path.join("__pycache__"),
    ];
    let mut count = 0u32;
    for t in targets {
        if t.is_dir() { std::fs::remove_dir_all(t).ok(); count += 1; }
    }
    if count == 0 {
        println!("  无构建产物可清理");
    } else {
        println!("  ✓ 已清理 {} 个构建目录", count);
    }
}
