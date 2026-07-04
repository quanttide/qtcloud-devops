/// doctor 命令：检查系统依赖的外部命令状态。
use std::process::Command;

pub fn status() {
    println!("系统诊断");
    println!("{}", "-".repeat(50));

    let checks: Vec<(&str, &[&str])> = vec![
        ("git", &["--version"]),
        ("gh", &["--version"]),
        ("cargo", &["--version"]),
        ("rustc", &["--version"]),
    ];

    for (cmd, args) in &checks {
        let status = check_command(cmd, args);
        println!("  {:<12} {}", cmd, status);
    }

    // gh 认证状态
    match Command::new("gh").args(["auth", "status"]).output() {
        Ok(out) if out.status.success() => {
            let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!(
                "  {:<12} ✅ {}",
                "gh-auth",
                msg.lines()
                    .nth(1)
                    .map(|l| l.trim())
                    .unwrap_or(msg.lines().next().unwrap_or(""))
            );
        }
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            println!(
                "  {:<12} ❌ {}",
                "gh-auth",
                msg.lines().next().unwrap_or("")
            );
        }
        Err(_) => {
            println!("  {:<12} ❌ 未安装", "gh-auth");
        }
    }
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
}
