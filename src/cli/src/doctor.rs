/// doctor 命令：检查系统依赖的外部命令状态。
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// 向 stdout 输出系统诊断信息。
pub fn status(repo_path: &Path) {
    let mut stdout = std::io::stdout();
    let _ = status_to(&mut stdout, repo_path);
}

/// 将系统诊断信息写入指定的 writer。
pub fn status_to(writer: &mut impl Write, repo_path: &Path) -> std::io::Result<()> {
    writeln!(writer, "系统诊断")?;
    writeln!(writer, "{}", "-".repeat(50))?;

    // 检测项目用到的语言
    let c = crate::contract::load(repo_path);
    let mut used_langs: Vec<String> = Vec::new();
    if c.scopes.is_empty() {
        used_langs = crate::contract::detect_all_languages(repo_path);
    } else {
        for s in &c.scopes {
            let scope_dir = repo_path.join(&s.dir);
            let mut langs = crate::contract::detect_all_languages(&scope_dir);
            used_langs.append(&mut langs);
        }
    }
    used_langs.sort();
    used_langs.dedup();

    // 始终显示的工具
    writeln!(
        writer,
        "  {:<12} {}",
        "git",
        check_command("git", &["--version"])
    )?;
    write_gh_status(writer)?;

    // 按语言显示工具链
    for lang in &["rust", "python", "go", "dart", "typescript"] {
        if !used_langs.iter().any(|l| l == lang) {
            continue;
        }
        match *lang {
            "rust" => {
                writeln!(
                    writer,
                    "  {:<12} {}",
                    "cargo",
                    check_command("cargo", &["--version"])
                )?;
                writeln!(
                    writer,
                    "  {:<12} {}",
                    "rustc",
                    check_command("rustc", &["--version"])
                )?;
            }
            "python" => {
                writeln!(
                    writer,
                    "  {:<12} {}",
                    "python",
                    check_command("python", &["--version"])
                )?;
                for sub in &["uv", "pytest", "coverage"] {
                    let v = check_command(sub, &["--version"]);
                    writeln!(writer, "    {:<10} {}", sub, v)?;
                }
            }
            "go" => {
                writeln!(
                    writer,
                    "  {:<12} {}",
                    "go",
                    check_command("go", &["version"])
                )?;
            }
            "dart" => {
                writeln!(
                    writer,
                    "  {:<12} {}",
                    "flutter",
                    check_command("flutter", &["--version"])
                )?;
                let v = check_command("dart", &["--version"]);
                writeln!(writer, "    {:<10} {}", "dart", v)?;
            }
            "typescript" => {
                writeln!(
                    writer,
                    "  {:<12} {}",
                    "node",
                    check_command("node", &["--version"])
                )?;
                for sub in &["npm", "npx"] {
                    let v = check_command(sub, &["--version"]);
                    writeln!(writer, "    {:<10} {}", sub, v)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn write_gh_status(writer: &mut impl Write) -> std::io::Result<()> {
    let ver = check_command("gh", &["--version"]);
    writeln!(writer, "  {:<12} {}", "gh", ver)?;
    match Command::new("gh").args(["auth", "status"]).output() {
        Ok(out) if out.status.success() => {
            let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let auth_line = msg.lines().nth(1).map(|l| l.trim()).unwrap_or("");
            writeln!(writer, "                  ✅ {}", auth_line)?;
        }
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            writeln!(
                writer,
                "                  ❌ {}",
                msg.lines().next().unwrap_or("")
            )?;
        }
        Err(_) => writeln!(writer, "                  ❌ 未登录")?,
    }
    Ok(())
}

fn check_command(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd).args(args).output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            format!("✅ {}", ver)
        }
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            format!("❌ {}", msg)
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => format!("❌ 未安装"),
            _ => format!("❌ {}", e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_git_exists() {
        let result = check_command("git", &["--version"]);
        assert!(result.starts_with("✅"), "git 应存在: {}", result);
    }

    #[test]
    fn test_check_nonexistent() {
        let result = check_command("nonexistent_cmd_xyz", &["--version"]);
        assert!(
            result.contains("未安装"),
            "不存在的命令应报未安装: {}",
            result
        );
    }

    #[test]
    fn test_status_to_output() {
        let d = tempfile::tempdir().unwrap();
        // 创建 Cargo.toml 让语言检测到 rust
        std::fs::write(d.path().join("Cargo.toml"), "[package]\n").unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");

        // 检查始终存在的结构元素
        assert!(output.contains("系统诊断"), "应包含标题");
        assert!(output.contains(&"-".repeat(50)), "应包含分隔线");
        assert!(output.contains("git"), "应包含 git");
        assert!(output.contains("gh"), "应包含 gh");
        // Rust 工具链（根据 Cargo.toml 检测）
        assert!(output.contains("cargo"), "应包含 cargo");
        assert!(output.contains("rustc"), "应包含 rustc");
    }
}
