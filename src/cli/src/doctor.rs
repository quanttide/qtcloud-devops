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
        if *cmd == "gh" {
            // gh：版本 + 认证状态作为子条目
            let ver = check_command("gh", &["--version"]);
            println!("  {:<12} {}", "gh", ver);
            match Command::new("gh").args(["auth", "status"]).output() {
                Ok(out) if out.status.success() => {
                    let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let auth_line = msg.lines().nth(1).map(|l| l.trim()).unwrap_or("");
                    println!("                  ✅ {}", auth_line);
                }
                Ok(out) => {
                    let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    println!("                  ❌ {}", msg.lines().next().unwrap_or(""));
                }
                Err(_) => {
                    println!("                  ❌ 未登录");
                }
            }
        } else {
            let status = check_command(cmd, args);
            println!("  {:<12} {}", cmd, status);
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
