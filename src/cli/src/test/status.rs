use std::path::Path;

use crate::contract;
use crate::test::Coverage;

/// 按 scope 输出测试状态（写 stdout 的便捷封装）。
pub fn status(repo_path: &Path, c: &contract::Contract) {
    let _ = status_to(&mut std::io::stdout(), repo_path, c);
}

/// 按 scope 输出测试状态，写入任意 writer。
pub fn status_to(
    writer: &mut impl std::io::Write,
    repo_path: &Path,
    c: &contract::Contract,
) -> std::io::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| repo_path.to_path_buf());
    let scopes: Vec<_> = c
        .scopes
        .iter()
        .filter(|s| {
            let scope_abs = repo_path.join(&s.dir);
            cwd.starts_with(&scope_abs) || scope_abs.starts_with(&cwd)
        })
        .collect();

    writeln!(writer, "测试状态")?;
    writeln!(writer, "{}", "-".repeat(50))?;

    if scopes.is_empty() {
        let lang = contract::detect_languages(repo_path)
            .into_iter()
            .next()
            .unwrap_or(contract::Language::Unknown(String::new()));
        let coverage =
            crate::test::coverage::collect_coverage(repo_path, &lang, c.stages.test.threshold);
        print_scope_status(writer, "(root)", &coverage)?;
    } else {
        for scope in &scopes {
            let scope_dir = repo_path.join(&scope.dir);
            if !scope_dir.exists() {
                writeln!(writer, "  [{}]     ⚠ 目录不存在", scope.name)?;
                continue;
            }
            let lang = c.resolve_language(scope, &scope_dir);
            let threshold = c.scope_test_threshold(scope);
            let coverage = crate::test::coverage::collect_coverage(&scope_dir, &lang, threshold);
            print_scope_status(writer, &scope.name, &coverage)?;
        }
    }

    Ok(())
}

pub(crate) fn print_scope_status(
    writer: &mut impl std::io::Write,
    name: &str,
    coverage: &Coverage,
) -> std::io::Result<()> {
    writeln!(writer, "  [{:<12}]", name)?;

    let cov_icon = if coverage.met() {
        "✅"
    } else if coverage.percentage > 0.0 {
        "⚠"
    } else {
        "—"
    };
    if coverage.percentage > 0.0 {
        writeln!(
            writer,
            "    覆盖率:       {:.1}%{}（阈值 {}%）",
            coverage.percentage, cov_icon, coverage.threshold,
        )?;
    } else {
        writeln!(writer, "    覆盖率:       未检测到覆盖率报告")?;
        writeln!(writer, "                  运行 `cargo llvm-cov --lcov --output-path target/coverage/lcov.info` 生成")?;
    }

    Ok(())
}
