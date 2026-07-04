/// doctor 命令：检查系统依赖的外部命令状态。
use std::io::Write;
use std::process::Command;

/// 向 stdout 输出系统诊断信息。
pub fn status() {
    let mut stdout = std::io::stdout();
    let _ = status_to(&mut stdout);
}

/// 将系统诊断信息写入指定的 writer。
pub fn status_to(writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer, "系统诊断")?;
    writeln!(writer, "{}", "-".repeat(50))?;

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
                Err(_) => {
                    writeln!(writer, "                  ❌ 未登录")?;
                }
            }
        } else {
            let status = check_command(cmd, args);
            writeln!(writer, "  {:<12} {}", cmd, status)?;
        }
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
        let mut buf = Vec::new();
        status_to(&mut buf).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");

        // 检查始终存在的结构元素
        assert!(output.contains("系统诊断"), "应包含标题");
        assert!(output.contains(&"-".repeat(50)), "应包含分隔线");

        // 检查所有命令名称都出现在输出中
        for cmd in &["git", "gh", "cargo", "rustc"] {
            assert!(output.contains(cmd), "应包含命令名 '{}'", cmd);
        }
    }
}
