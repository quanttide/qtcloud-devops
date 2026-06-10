use std::path::Path;

/// 在指定路径初始化 git 仓库并设置 user.name/user.email。
pub fn git_init(path: &Path) {
    std::process::Command::new("git")
        .args(["init", "-b", "main"]).current_dir(path).output().unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"]).current_dir(path).output().unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
}

/// 在已初始化的 git 仓库中创建一次提交（写入 file + add + commit）。
pub fn git_commit(path: &Path, msg: &str) {
    std::fs::write(path.join("file"), msg).unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(path).output().unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", msg]).current_dir(path).output().unwrap();
}

/// 创建带子模块的父仓库，返回父仓库路径。
pub fn setup_repo_with_submodule(tmp: &Path) -> std::path::PathBuf {
    let parent = tmp.join("parent");
    let sub = tmp.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    git_init(&sub);
    git_commit(&sub, "init sub");
    std::fs::create_dir_all(&parent).unwrap();
    git_init(&parent);
    git_commit(&parent, "init parent");
    std::process::Command::new("git")
        .args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"])
        .current_dir(&parent).output().unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "add submodule"])
        .current_dir(&parent).output().unwrap();
    parent
}
