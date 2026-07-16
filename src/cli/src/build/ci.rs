use crate::build::CiRun;

/// 解析 CI workflow 名称。ci_workflow 优先，无则按约定 build-{scope}。
pub(crate) fn resolve_workflow(scope: &str, ci_workflow: Option<&str>) -> String {
    match ci_workflow {
        Some(w) => w.to_string(),
        None => format!("build-{}", scope),
    }
}

/// 从 `gh run list --json conclusion,displayTitle,headBranch,number` 的输出解析运行记录。
///
/// 输入格式：`[{"conclusion":"success","displayTitle":"CI","headBranch":"main","number":42}]`
/// 返回 None 表示无有效记录（空数组或格式异常）。
pub(crate) fn parse_gh_run_list(output: &str) -> Option<CiRun> {
    let conclusion = output
        .split("\"conclusion\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))?;
    if conclusion.is_empty() {
        return None;
    }
    let title = output
        .split("\"displayTitle\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .unwrap_or("");
    let branch = output
        .split("\"headBranch\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .unwrap_or("?");
    let number: String = output
        .split("\"number\":")
        .nth(1)
        .map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect())
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "?".into());

    Some(CiRun {
        conclusion: conclusion.to_string(),
        title: title.to_string(),
        branch: branch.to_string(),
        number,
    })
}

pub(crate) fn check_ci(scope: &str, ci_workflow: Option<&str>) -> String {
    let workflow = resolve_workflow(scope, ci_workflow);
    let output = match std::process::Command::new("gh")
        .args([
            "run",
            "list",
            "--limit",
            "1",
            "--workflow",
            &workflow,
            "--json",
            "conclusion,displayTitle,headBranch,number",
        ])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        Ok(_) => return "⚠ 无 CI 运行记录".into(),
        Err(_) => return "⚠ gh CLI 未安装".into(),
    };

    let out = String::from_utf8_lossy(&output);
    match parse_gh_run_list(&out) {
        Some(run) => match run.conclusion.as_str() {
            "success" => format!("✅ {} ({} #{})", run.title, run.branch, run.number),
            "failure" => format!("❌ {} ({} #{})", run.title, run.branch, run.number),
            "cancelled" => format!("🔶 {} 已取消", run.title),
            s => format!("⏳ {} ({}) - {}", run.title, run.branch, s),
        },
        None => "⚠ 无 CI 运行记录".into(),
    }
}
