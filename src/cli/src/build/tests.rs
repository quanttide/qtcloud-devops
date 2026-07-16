#[cfg(test)]
mod tests {
    use crate::build::*;
    use crate::build::status::*;
    use crate::build::ci::*;
    use crate::build::check::*;
    use crate::contract;

    use std::path::Path;

    #[test]
    fn test_print_scope_all_ok() {
        let d = tempfile::tempdir().unwrap();
        let c = contract::load(d.path());
        let vs = contract::VersionState {
            tag_version: Some("0.1.0".into()),
            config_version: Some("0.1.0".into()),
            consistent: true,
            config_files: vec![("Cargo.toml".into(), Some("0.1.0".into()))],
        };
        let release = contract::StageRelease::default();
        let s = build_scope_str(&ScopeInfo {
            name: "test", dir: d.path(), lang: &contract::Language::Rust, c: &c, vs: &vs, release: &release,
        });
        assert!(s.contains("✅"), "一致状态应显示 ✅");
    }

    #[test]
    fn test_print_scope_version_inconsistent() {
        let vs = contract::VersionState {
            tag_version: Some("0.2.0".into()),
            config_version: Some("0.1.0".into()),
            consistent: false,
            config_files: vec![("Cargo.toml".into(), Some("0.1.0".into()))],
        };
        let release = contract::StageRelease::default();
        let c = contract::Contract::default();
        let s = build_scope_str(&ScopeInfo {
            name: "test", dir: Path::new("/tmp"), lang: &contract::Language::Rust, c: &c, vs: &vs, release: &release,
        });
        assert!(s.contains("配置不一致"), "应显示不一致");
    }

    #[test]
    fn test_print_scope_tag_without_config() {
        let vs = contract::VersionState {
            tag_version: Some("0.1.0".into()),
            config_version: None,
            consistent: false,
            config_files: vec![("Cargo.toml".into(), None)],
        };
        let release = contract::StageRelease::default();
        let c = contract::Contract::default();
        let s = build_scope_str(&ScopeInfo {
            name: "test", dir: Path::new("/tmp"), lang: &contract::Language::Rust, c: &c, vs: &vs, release: &release,
        });
        assert!(s.contains("无配置文件"), "有 tag 无配置应提示无配置文件");
    }

    #[test]
    fn test_print_scope_config_without_tag() {
        let vs = contract::VersionState {
            tag_version: None,
            config_version: Some("0.1.0".into()),
            consistent: false,
            config_files: vec![("Cargo.toml".into(), Some("0.1.0".into()))],
        };
        let release = contract::StageRelease::default();
        let c = contract::Contract::default();
        let s = build_scope_str(&ScopeInfo {
            name: "test", dir: Path::new("/tmp"), lang: &contract::Language::Rust, c: &c, vs: &vs, release: &release,
        });
        assert!(s.contains("有配置版本"), "有配置无 tag 应提示");
    }

    #[test]
    fn test_print_scope_no_release() {
        let vs = contract::VersionState {
            tag_version: None,
            config_version: None,
            consistent: false,
            config_files: vec![],
        };
        let release = contract::StageRelease::default();
        let c = contract::Contract::default();
        let s = build_scope_str(&ScopeInfo {
            name: "test", dir: Path::new("/tmp"), lang: &contract::Language::Rust, c: &c, vs: &vs, release: &release,
        });
        assert!(s.contains("暂无发布"), "无 tag 无配置应显示暂无发布");
    }

    #[test]
    fn test_is_working_tree_dirty_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        assert!(!is_working_tree_dirty(d.path()));
    }

    #[test]
    fn test_resolve_workflow_default() {
        assert_eq!(resolve_workflow("cli", None), "build-cli");
        assert_eq!(resolve_workflow("studio", None), "build-studio");
    }

    #[test]
    fn test_resolve_workflow_custom() {
        assert_eq!(resolve_workflow("cli", Some("my-pipeline")), "my-pipeline");
        assert_eq!(resolve_workflow("cli", Some("release-ci")), "release-ci");
    }

    #[test]
    fn test_detect_no_contract_yaml() {
        let d = tempfile::tempdir().unwrap();
        let c = contract::load(d.path());
        assert!(c.scopes.is_empty());
    }

    // ── parse_gh_run_list ─────────────────────────────────────

    #[test]
    fn test_parse_gh_run_list_success() {
        let out =
            r#"[{"conclusion":"success","displayTitle":"CI","headBranch":"main","number":42}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "success");
        assert_eq!(run.title, "CI");
        assert_eq!(run.branch, "main");
        assert_eq!(run.number, "42");
    }

    #[test]
    fn test_parse_gh_run_list_failure() {
        let out =
            r#"[{"conclusion":"failure","displayTitle":"Build","headBranch":"feat/x","number":7}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "failure");
        assert_eq!(run.title, "Build");
        assert_eq!(run.branch, "feat/x");
        assert_eq!(run.number, "7");
    }

    #[test]
    fn test_parse_gh_run_list_cancelled() {
        let out =
            r#"[{"conclusion":"cancelled","displayTitle":"CI","headBranch":"main","number":99}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "cancelled");
        assert_eq!(run.number, "99");
    }

    #[test]
    fn test_parse_gh_run_list_empty_array() {
        assert!(parse_gh_run_list("[]").is_none());
    }

    #[test]
    fn test_parse_gh_run_list_empty_stdout() {
        assert!(parse_gh_run_list("").is_none());
    }

    #[test]
    fn test_parse_gh_run_list_no_number() {
        // 一些旧版本 gh 可能不返回 number
        let out = r#"[{"conclusion":"success","displayTitle":"CI","headBranch":"main"}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.number, "?");
    }

    #[test]
    fn test_parse_gh_run_list_unknown_conclusion() {
        let out =
            r#"[{"conclusion":"neutral","displayTitle":"Check","headBranch":"main","number":1}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "neutral");
        assert_eq!(run.title, "Check");
    }

    #[test]
    fn test_parse_gh_run_list_large_input() {
        use std::time::Instant;
        // 生成 1000 条 CI 运行记录 JSON
        let mut items = Vec::with_capacity(1000);
        for i in 0..1000 {
            items.push(format!(
                r#"{{"conclusion":"success","displayTitle":"CI","headBranch":"main","number":{}}}"#,
                i
            ));
        }
        let out = format!("[{}]", items.join(","));
        let start = Instant::now();
        let run = parse_gh_run_list(&out);
        let elapsed = start.elapsed();
        assert!(run.is_some(), "应解析成功");
        assert_eq!(run.as_ref().unwrap().number, "0");
        assert!(
            elapsed.as_micros() < 5000,
            "1000 条记录应在 5ms 内解析，实际: {}μs",
            elapsed.as_micros()
        );
    }

    // ── check_command ─────────────────────────────────────────

    #[test]
    fn test_check_command_all_languages() {
        assert_eq!(
            check_command(&contract::Language::Rust),
            Some(("cargo", "cargo check"))
        );
        assert_eq!(
            check_command(&contract::Language::Python),
            Some(("uv", "uv check"))
        );
        assert_eq!(
            check_command(&contract::Language::Go),
            Some(("go", "go vet"))
        );
        assert_eq!(
            check_command(&contract::Language::Dart),
            Some(("dart", "dart analyze"))
        );
        assert_eq!(
            check_command(&contract::Language::TypeScript),
            Some(("npx", "tsc --noEmit"))
        );
        assert_eq!(
            check_command(&contract::Language::Unknown("?".into())),
            None
        );
    }

    // ── check_manifest_file ────────────────────────────────────

    #[test]
    fn test_check_manifest_file_all_languages() {
        assert_eq!(
            check_manifest_file(&contract::Language::Rust),
            Some("Cargo.toml")
        );
        assert_eq!(
            check_manifest_file(&contract::Language::Python),
            Some("pyproject.toml")
        );
        assert_eq!(check_manifest_file(&contract::Language::Go), Some("go.mod"));
        assert_eq!(
            check_manifest_file(&contract::Language::Dart),
            Some("pubspec.yaml")
        );
        assert_eq!(
            check_manifest_file(&contract::Language::TypeScript),
            Some("package.json")
        );
        assert_eq!(
            check_manifest_file(&contract::Language::Unknown("?".into())),
            None
        );
    }

    // ── check_args ─────────────────────────────────────────────

    #[test]
    fn test_check_args_rust_includes_manifest_path() {
        let d = tempfile::tempdir().unwrap();
        let args = check_args(&contract::Language::Rust, d.path()).unwrap();
        assert!(args.contains(&"check".to_string()));
        assert!(args.iter().any(|a| a.contains("Cargo.toml")));
    }

    #[test]
    fn test_check_args_unknown_returns_none() {
        let d = tempfile::tempdir().unwrap();
        assert!(check_args(&contract::Language::Unknown("?".into()), d.path()).is_none());
    }

    // ── check_dependencies ──────────────────────────────────────

    #[test]
    fn test_check_deps_clean() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[dependencies]\nserde = \"1\"\n",
        )
        .unwrap();
        let r = check_dependencies(d.path());
        assert!(r.contains("✅"), "应返回干净: {}", r);
    }

    #[test]
    fn test_check_deps_path_dep() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[dependencies]\nfoo = { path = \"../local\" }\n",
        )
        .unwrap();
        let r = check_dependencies(d.path());
        assert!(r.contains("⚠"), "应检测到 path 依赖: {}", r);
    }

    #[test]
    fn test_check_deps_git_no_rev() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[dependencies]\nbar = { git = \"https://github.com/foo/bar\" }\n",
        )
        .unwrap();
        let r = check_dependencies(d.path());
        assert!(r.contains("⚠"), "应检测到 git 无 rev: {}", r);
    }

    #[test]
    fn test_check_deps_no_cargo_toml() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(check_dependencies(d.path()), "—");
    }
}
