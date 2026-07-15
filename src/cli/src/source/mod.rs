/// source 命令：检查系统依赖的外部命令状态。
pub mod changelog;
pub mod gh;
pub mod git;
pub mod tag;

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
    let used_langs = detect_used_languages(repo_path);
    let mut o = build_tool_status_header(&used_langs);
    o.push_str(&build_language_sections(&used_langs));
    write!(writer, "{}", o)
}

fn detect_used_languages(repo_path: &Path) -> Vec<String> {
    let c = crate::contract::load(repo_path);
    let mut used_langs: Vec<String> = Vec::new();
    if c.scopes.is_empty() {
        for lang in crate::contract::detect_languages(repo_path) {
            used_langs.push(lang.as_str().to_string());
        }
    } else {
        for s in &c.scopes {
            for lang in crate::contract::detect_languages(&repo_path.join(&s.dir)) {
                used_langs.push(lang.as_str().to_string());
            }
        }
    }
    used_langs.sort();
    used_langs.dedup();
    used_langs
}

fn build_tool_status_header(_used_langs: &[String]) -> String {
    let mut o = format!("系统诊断\n{}\n", "-".repeat(50));
    o.push_str(&format!(
        "  {:<12} {}\n",
        "git",
        check_command("git", &["--version"])
    ));
    let gh_ver = check_command("gh", &["--version"]);
    o.push_str(&format!("  {:<12} {}\n", "gh", gh_ver));
    if gh_ver.starts_with("✅") {
        o.push_str(&format!(
            "    {:<10} {}\n",
            "auth",
            check_command("gh", &["auth", "status"])
        ));
    }
    o
}

fn build_language_sections(used_langs: &[String]) -> String {
    let mut o = String::new();
    for lang in &["rust", "python", "go", "dart", "typescript"] {
        if !used_langs.iter().any(|l| l == lang) {
            continue;
        }
        o.push_str(&match *lang {
            "rust" => format!(
                "  {:<12} {}\n  {:<12} {}\n",
                "cargo",
                check_command("cargo", &["--version"]),
                "rustc",
                check_command("rustc", &["--version"]),
            ),
            "python" => {
                let mut s = format!(
                    "  {:<12} {}\n",
                    "python",
                    check_command("python", &["--version"])
                );
                for sub in &["uv", "pytest", "coverage"] {
                    s.push_str(&format!(
                        "    {:<10} {}\n",
                        sub,
                        check_command(sub, &["--version"])
                    ));
                }
                s
            }
            "go" => format!("  {:<12} {}\n", "go", check_command("go", &["version"])),
            "dart" => format!(
                "  {:<12} {}\n    {:<10} {}\n",
                "flutter",
                check_command("flutter", &["--version"]),
                "dart",
                check_command("dart", &["--version"]),
            ),
            "typescript" => {
                let mut s = format!(
                    "  {:<12} {}\n",
                    "node",
                    check_command("node", &["--version"])
                );
                for sub in &["npm", "npx"] {
                    s.push_str(&format!(
                        "    {:<10} {}\n",
                        sub,
                        check_command(sub, &["--version"])
                    ));
                }
                s
            }
            _ => String::new(),
        });
    }
    o
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
    fn test_status_to_python() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("pyproject.toml"),
            "[project]\nname = \"test\"\n",
        )
        .unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");
        assert!(output.contains("git"), "应包含 git");
        assert!(output.contains("python"), "Python 项目应显示 python");
    }

    #[test]
    fn test_status_to_go() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("go.mod"), "module test\n").unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");
        assert!(output.contains("go"), "Go 项目应显示 go 工具链");
    }

    #[test]
    fn test_status_to_typescript() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("package.json"), "{\"name\":\"test\"}\n").unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");
        assert!(output.contains("node"), "TS 项目应显示 node 工具链");
    }

    #[test]
    fn test_status_to_dart() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("pubspec.yaml"), "name: test\n").unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");
        assert!(output.contains("flutter"), "Dart 项目应显示 flutter 工具链");
    }

    #[test]
    fn test_status_to_no_lang() {
        let d = tempfile::tempdir().unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");
        assert!(output.contains("git"), "应始终显示 git");
        assert!(output.contains("gh"), "应始终显示 gh");
    }

    #[test]
    fn test_status_to_output() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Cargo.toml"), "[package]\n").unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8(buf).expect("非 UTF-8 输出");
        assert!(output.contains("系统诊断"), "应包含标题");
        assert!(output.contains(&"-".repeat(50)), "应包含分隔线");
        assert!(output.contains("git"), "应包含 git");
        assert!(output.contains("gh"), "应包含 gh");
        assert!(output.contains("cargo"), "应包含 cargo");
        assert!(output.contains("rustc"), "应包含 rustc");
    }
}
