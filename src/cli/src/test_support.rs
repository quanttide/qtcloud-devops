/// 共享测试工具：git 仓库初始化、提交等。
///
/// 仅在测试时编译，减少各模块重复的 git_init / git_commit。
use std::path::Path;
use std::process::Command;

/// 初始化 git 仓库（`main` 分支），配置 user 信息。
pub fn git_init(path: &Path) {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "t@t"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "t"])
        .current_dir(path)
        .output()
        .unwrap();
}

/// 在当前仓库中创建并提交文件。
pub fn git_commit(path: &Path, msg: &str) {
    std::fs::write(path.join("f"), msg).unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(path)
        .output()
        .unwrap();
}
