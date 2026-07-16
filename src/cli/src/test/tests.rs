#[cfg(test)]
mod tests {
    use crate::test::*;
    use crate::test::audit::*;
    use crate::test::coverage::*;
    use crate::test::run::*;
    use crate::test::status::*;
    use crate::test::summary::*;
    use crate::contract;

    use std::path::Path;

    #[test]
    fn test_parse_test_summary_ok() {
        let s = parse_test_summary(
            "test result: ok. 10 passed; 0 failed; 2 ignored; 0 measured; 12 filtered out",
        );
        assert_eq!(s.passed, 10);
        assert_eq!(s.failed, 0);
        assert_eq!(s.skipped, 2);
        assert_eq!(s.total, 12);
    }

    #[test]
    fn test_parse_test_summary_failed() {
        let s =
            parse_test_summary("test result: FAILED. 8 passed; 3 failed; 1 ignored; 0 measured");
        assert_eq!(s.passed, 8);
        assert_eq!(s.failed, 3);
        assert_eq!(s.skipped, 1);
    }

    // ── is_error_enum_declaration ─────────────────────────────

    #[test]
    fn test_is_error_enum_declaration_matches() {
        assert_eq!(
            is_error_enum_declaration("pub enum MyError {"),
            Some("MyError".into())
        );
    }

    #[test]
    fn test_is_error_enum_declaration_matches_lowercase() {
        assert_eq!(
            is_error_enum_declaration("pub enum parse_error {"),
            Some("parse_error".into())
        );
    }

    #[test]
    fn test_is_error_enum_declaration_skips_non_error() {
        assert_eq!(is_error_enum_declaration("pub enum Color {"), None);
    }

    #[test]
    fn test_is_error_enum_declaration_skips_non_pub() {
        assert_eq!(is_error_enum_declaration("enum MyError {"), None);
    }

    #[test]
    fn test_is_error_enum_declaration_skips_struct() {
        assert_eq!(is_error_enum_declaration("pub struct Error {"), None);
    }

    #[test]
    fn test_is_error_enum_declaration_extracts_name_with_generics() {
        assert_eq!(
            is_error_enum_declaration("pub enum MyError<E> {"),
            Some("MyError<E>".into())
        );
    }

    // ── extract_variant_name ────────────────────────────────────

    #[test]
    fn test_extract_variant_name_simple() {
        assert_eq!(
            extract_variant_name("    IoError,"),
            Some("IoError".into())
        );
    }

    #[test]
    fn test_extract_variant_name_with_tuple() {
        assert_eq!(
            extract_variant_name("    NotFound(String),"),
            Some("NotFound".into())
        );
    }

    #[test]
    fn test_extract_variant_name_with_braces() {
        assert_eq!(
            extract_variant_name("    Detailed { code: u32 },"),
            Some("Detailed".into())
        );
    }

    #[test]
    fn test_extract_variant_name_skips_comment() {
        assert_eq!(extract_variant_name("// this is a comment"), None);
    }

    #[test]
    fn test_extract_variant_name_skips_derive() {
        assert_eq!(
            extract_variant_name("#[derive(Debug)]"),
            None
        );
    }

    #[test]
    fn test_extract_variant_name_skips_empty() {
        assert_eq!(extract_variant_name(""), None);
    }

    #[test]
    fn test_extract_variant_name_skips_blank() {
        assert_eq!(extract_variant_name("   "), None);
    }

    #[test]
    fn test_parse_lcov_empty() {
        assert!(parse_lcov_coverage("").is_none());
    }

    #[test]
    fn test_parse_lcov_simple() {
        let content = "SF:src/lib.rs\nDA:1,1\nDA:2,0\nDA:3,1\nend_of_record\n";
        let pct = parse_lcov_coverage(content).unwrap();
        assert!((pct - 66.666).abs() < 0.01);
    }

    #[test]
    fn test_print_scope_coverage_warn() {
        let mut buf = Vec::new();
        let c = Coverage {
            percentage: 50.0,
            threshold: 70.0,
        };
        print_scope_status(&mut buf, "test", &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("⚠"), "低于阈值应有 ⚠");
    }

    #[test]
    fn test_coverage_met() {
        let c = Coverage {
            percentage: 80.0,
            threshold: 70.0,
        };
        assert!(c.met());
    }

    // ── 性能测试（大输入边界） ──────────────────────────────────

    #[test]
    fn test_parse_test_summary_large_output() {
        // 模拟 1000 组测试结果
        let mut content = String::new();
        for i in 0..500 {
            content.push_str(&format!(
                "test test_{i} ... ok\ntest test_{i}_a ... FAILED\n"
            ));
        }
        content.push_str("test result: FAILED. 500 passed; 500 failed; 0 ignored; 0 measured\n");
        let s = parse_test_summary(&content);
        assert_eq!(s.passed, 500);
        assert_eq!(s.failed, 500);
        assert_eq!(s.total, 1000);
    }

    #[test]
    fn test_parse_lcov_large_input() {
        // 10000 DA 行的 lcov 输出
        let mut lines = vec!["SF:src/lib.rs".to_string()];
        for i in 0..5000 {
            lines.push(format!("DA:{},1", i + 1));
            lines.push(format!("DA:{},0", i + 5001));
        }
        lines.push("end_of_record".to_string());
        let content = lines.join("\n");
        let pct = parse_lcov_coverage(&content).unwrap();
        assert!((pct - 50.0).abs() < 0.01, "10000 行应正确解析为 50%");
    }

    #[test]
    fn test_parse_test_summary_very_large_stdout() {
        // 大量非测试行（CI 日志、warnings）中间夹杂测试结果
        let mut lines: Vec<String> = (0..2000)
            .map(|i| format!("  Compiling crate-{} v0.1.0", i))
            .collect();
        lines.push("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured".into());
        for i in 2000..4000 {
            lines.push(format!("warning: unused variable `x` in crate-{}", i));
        }
        let content = lines.join("\n");
        let start = std::time::Instant::now();
        let s = parse_test_summary(&content);
        let elapsed = start.elapsed();
        assert_eq!(s.passed, 1);
        assert!(
            elapsed.as_millis() < 500,
            "4000 行日志应在 500ms 内解析完成: {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_parse_lcov_no_match() {
        // 大量不相关行，没有 DA: 前缀
        let mut lines = vec!["TN:".to_string()];
        for i in 0..5000 {
            lines.push(format!("SF:src/file_{i}.rs"));
            lines.push("end_of_record".to_string());
        }
        let content = lines.join("\n");
        assert!(parse_lcov_coverage(&content).is_none());
    }

    #[test]
    fn test_parse_cobertura_simple() {
        let content = r#"<coverage line-rate="0.85"></coverage>"#;
        let pct = parse_cobertura_coverage(content).unwrap();
        assert!((pct - 85.0).abs() < 0.01);
    }

    #[test]
    fn test_coverage_not_met() {
        let c = Coverage {
            percentage: 60.0,
            threshold: 70.0,
        };
        assert!(!c.met());
    }

    // ── test_command ──────────────────────────────────────────

    #[test]
    fn test_command_all_languages() {
        assert_eq!(
            test_command(&contract::Language::Rust),
            Some(("cargo", &["test"][..]))
        );
        assert_eq!(
            test_command(&contract::Language::Python),
            Some(("python", &["-m", "pytest"][..]))
        );
        assert_eq!(
            test_command(&contract::Language::Go),
            Some(("go", &["test", "./..."][..]))
        );
        assert_eq!(
            test_command(&contract::Language::Dart),
            Some(("flutter", &["test"][..]))
        );
        assert_eq!(
            test_command(&contract::Language::TypeScript),
            Some(("npm", &["test"][..]))
        );
        assert_eq!(test_command(&contract::Language::Unknown("?".into())), None);
    }

    // ── coverage_command ──────────────────────────────────

    #[test]
    fn test_coverage_command_all_languages() {
        assert_eq!(
            coverage_command(&contract::Language::Rust).map(|(c, _)| c),
            Some("cargo")
        );
        assert_eq!(
            coverage_command(&contract::Language::Python).map(|(c, _)| c),
            Some("coverage")
        );
        assert_eq!(
            coverage_command(&contract::Language::Go).map(|(c, _)| c),
            Some("go")
        );
        assert_eq!(
            coverage_command(&contract::Language::Dart).map(|(c, _)| c),
            Some("flutter")
        );
        assert_eq!(
            coverage_command(&contract::Language::TypeScript).map(|(c, _)| c),
            Some("npx")
        );
        assert!(coverage_command(&contract::Language::Unknown("auto".into())).is_none());
    }

    // ── test_manifest_file ────────────────────────────────────

    #[test]
    fn test_manifest_file_all_languages() {
        assert_eq!(
            test_manifest_file(&contract::Language::Rust),
            Some("Cargo.toml")
        );
        assert_eq!(
            test_manifest_file(&contract::Language::Python),
            Some("pyproject.toml")
        );
        assert_eq!(test_manifest_file(&contract::Language::Go), Some("go.mod"));
        assert_eq!(
            test_manifest_file(&contract::Language::Dart),
            Some("pubspec.yaml")
        );
        assert_eq!(
            test_manifest_file(&contract::Language::TypeScript),
            Some("package.json")
        );
        assert_eq!(
            test_manifest_file(&contract::Language::Unknown("?".into())),
            None
        );
    }

    // ── cache_path ────────────────────────────────────────────

    #[test]
    fn test_cache_path_resolves_in_dir() {
        let d = tempfile::tempdir().unwrap();
        let p = cache_path(d.path());
        assert!(p.ends_with(".quanttide/devops/test-summary.json"));
    }

    #[test]
    fn test_cache_path_absolute() {
        let p = cache_path(Path::new("/tmp/myproject"));
        assert_eq!(
            p,
            Path::new("/tmp/myproject/.quanttide/devops/test-summary.json")
        );
    }

    // ── save / collect / clear cache ─────────────────────────

    #[test]
    fn test_save_and_collect_cache_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let summary = TestSummary {
            total: 42,
            passed: 40,
            failed: 1,
            skipped: 1,
        };
        save_test_summary(d.path(), &summary);
        let cached = collect_test_summary(d.path(), &contract::Language::Rust);
        assert_eq!(cached.total, 42);
        assert_eq!(cached.passed, 40);
        assert_eq!(cached.failed, 1);
        assert_eq!(cached.skipped, 1);
    }

    #[test]
    fn test_collect_cache_nonexistent_returns_default() {
        let d = tempfile::tempdir().unwrap();
        let summary = collect_test_summary(d.path(), &contract::Language::Rust);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.passed, 0);
    }

    #[test]
    fn test_clear_cache_removes_file() {
        let d = tempfile::tempdir().unwrap();
        save_test_summary(
            d.path(),
            &TestSummary {
                total: 5,
                ..Default::default()
            },
        );
        assert!(cache_path(d.path()).exists());
        clear_cache(d.path());
        assert!(!cache_path(d.path()).exists());
    }

    // ── parse_test_summary 边缘情况 ───────────────────────

    #[test]
    fn test_parse_test_summary_empty() {
        let s = parse_test_summary("");
        assert_eq!(s.total, 0);
    }

    #[test]
    fn test_parse_test_summary_no_result_line() {
        let s = parse_test_summary("Compiling foo ...\n   Compiling bar ...\n");
        assert_eq!(s.total, 0);
    }

    #[test]
    fn test_parse_test_summary_malformed_skips_bad_tokens() {
        // 'abc' 不是合法数字，应跳过
        let s = parse_test_summary("test result: ok. abc passed; 0 failed");
        assert_eq!(s.passed, 0);
        assert_eq!(s.failed, 0);
    }

    #[test]
    fn test_parse_test_summary_multiple_result_lines() {
        // 工作空间多 crate 场景：每行一个 test result
        let content = "test result: ok. 5 passed; 0 failed; 1 ignored\n\
                       test result: ok. 3 passed; 1 failed; 0 ignored\n";
        let s = parse_test_summary(content);
        assert_eq!(s.passed, 8);
        assert_eq!(s.failed, 1);
        assert_eq!(s.skipped, 1);
        assert_eq!(s.total, 10);
    }

    // ── collect_coverage 不存在文件时的行为 ────────────────

    #[test]
    fn test_collect_coverage_no_file_rust() {
        let d = tempfile::tempdir().unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Rust, 70.0);
        assert_eq!(cov.percentage, 0.0);
        assert_eq!(cov.threshold, 70.0);
        assert!(!cov.met());
    }

    #[test]
    fn test_collect_coverage_unknown_lang_no_paths() {
        let d = tempfile::tempdir().unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Unknown("x".into()), 80.0);
        assert_eq!(cov.percentage, 0.0);
        assert_eq!(cov.threshold, 80.0);
    }

    #[test]
    fn test_collect_coverage_rust_with_lcov_file() {
        let d = tempfile::tempdir().unwrap();
        let cov_dir = d.path().join("target/coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/lib.rs\nDA:1,1\nDA:2,0\nDA:3,1\nend_of_record\n",
        )
        .unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Rust, 70.0);
        assert!((cov.percentage - 66.666).abs() < 0.01);
        assert!(!cov.met());
    }

    #[test]
    fn test_collect_coverage_python_with_cobertura() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("coverage.xml"),
            r#"<coverage line-rate="0.92"></coverage>"#,
        )
        .unwrap();
        let cov = collect_coverage(d.path(), &contract::Language::Python, 80.0);
        assert!((cov.percentage - 92.0).abs() < 0.01);
        assert!(cov.met());
    }

    // ── TestSummary 序列化/反序列化 ────────────────────────

    #[test]
    fn test_test_summary_serde_roundtrip() {
        let s = TestSummary {
            total: 100,
            passed: 90,
            failed: 5,
            skipped: 5,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: TestSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total, 100);
        assert_eq!(back.passed, 90);
        assert_eq!(back.failed, 5);
        assert_eq!(back.skipped, 5);
    }

    #[test]
    fn test_test_summary_serde_default_roundtrip() {
        let s = TestSummary::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: TestSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total, 0);
    }

    // ── Coverage 方法 ─────────────────────────────────────────

    #[test]
    fn test_coverage_met_exact() {
        let c = Coverage {
            percentage: 70.0,
            threshold: 70.0,
        };
        assert!(c.met());
    }

    #[test]
    fn test_coverage_met_zero_threshold() {
        let c = Coverage {
            percentage: 0.0,
            threshold: 0.0,
        };
        assert!(c.met());
    }

    // ── print_scope 更多变体 ───────────────────────────────

    #[test]
    fn test_print_scope_all_passed_no_coverage() {
        let mut buf = Vec::new();
        let c = Coverage {
            percentage: 0.0,
            threshold: 70.0,
        };
        print_scope_status(&mut buf, "core", &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("未检测到覆盖率报告"));
    }

    #[test]
    fn test_print_scope_with_coverage_met() {
        let mut buf = Vec::new();
        let c = Coverage {
            percentage: 85.0,
            threshold: 70.0,
        };
        print_scope_status(&mut buf, "lib", &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("✅"), "满足阈值应有 ✅");
        assert!(out.contains("85.0%"));
    }

    #[test]
    fn test_print_scope_coverage_below_threshold() {
        let mut buf = Vec::new();
        let c = Coverage {
            percentage: 30.0,
            threshold: 70.0,
        };
        print_scope_status(&mut buf, "lib", &c).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("⚠"), "低于阈值应有 ⚠");
        assert!(out.contains("30.0%"));
    }

    // ── parse_test_summary 更多变体 ─────────────────────────

    #[test]
    fn test_parse_test_summary_filtered_out() {
        let s =
            parse_test_summary("test result: ok. 5 passed; 0 failed; 0 ignored; 50 filtered out");
        assert_eq!(s.total, 5);
        assert_eq!(s.passed, 5);
    }

    #[test]
    fn test_parse_test_summary_with_measured() {
        let s = parse_test_summary("test result: ok. 3 passed; 1 failed; 0 ignored; 2 measured");
        assert_eq!(s.total, 4);
        assert_eq!(s.passed, 3);
        assert_eq!(s.failed, 1);
    }

    #[test]
    fn test_parse_test_summary_zero_all() {
        let s = parse_test_summary("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured");
        assert_eq!(s.total, 0);
    }

    // ── parse_lcov_coverage 边缘 ─────────────────────────────

    #[test]
    fn test_parse_lcov_da_with_non_numeric_count() {
        // count 不是数字，行仍计为 total_lines 但不计入 hit
        let content = "DA:1,abc\nDA:2,1\nend_of_record\n";
        let pct = parse_lcov_coverage(content).unwrap();
        assert!((pct - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_lcov_multiple_records() {
        // 多个 end_of_record 块，只累计 DA 行
        let content = "DA:1,1\nend_of_record\nSF:other.rs\nDA:2,0\nend_of_record\n";
        let pct = parse_lcov_coverage(content).unwrap();
        assert!((pct - 50.0).abs() < 0.01);
    }

    // ── parse_cobertura_coverage 边缘 ────────────────────────

    #[test]
    fn test_parse_cobertura_no_match() {
        assert!(parse_cobertura_coverage("<html></html>").is_none());
    }

    #[test]
    fn test_parse_cobertura_no_line_rate() {
        assert!(parse_cobertura_coverage(r#"<coverage branch-rate="0.5"></coverage>"#).is_none());
    }

    #[test]
    fn test_parse_cobertura_bad_line_rate() {
        assert!(parse_cobertura_coverage(r#"<coverage line-rate="abc"></coverage>"#).is_none());
    }

    #[test]
    fn test_parse_cobertura_large_xml() {
        use std::time::Instant;
        let mut lines = vec![r#"<coverage line-rate="0.85">"#.to_string()];
        for i in 0..5000 {
            lines.push(format!(r#"<package name="pkg-{i}" line-rate="0.9"><class name="Cls{i}" filename="src/file{i}.rs" line-rate="0.9"/></package>"#));
        }
        lines.push("</coverage>".to_string());
        let content = lines.join("\n");
        let start = Instant::now();
        let pct = parse_cobertura_coverage(&content);
        let elapsed = start.elapsed();
        assert!((pct.unwrap() - 85.0).abs() < 0.01);
        assert!(
            elapsed.as_micros() < 5000,
            "5000 行 Cobertura 应在 5ms 内解析，实际: {}μs",
            elapsed.as_micros()
        );
    }

    // ── TestSummary serde 零值 ─────────────────────────────

    #[test]
    fn test_test_summary_serde_all_zero() {
        let s = TestSummary {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: TestSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total, 0);
    }

    // ── scope 过滤性能 ────────────────────────────────────────

    #[test]
    fn test_scope_filter_large_contract() {
        use std::time::Instant;
        let repo_path = Path::new("/tmp/repo");
        let cwd = Path::new("/tmp/repo/packages/cli");
        let mut scopes = Vec::new();
        for i in 0..1000 {
            scopes.push(contract::Scope {
                name: format!("scope-{}", i),
                dir: format!("packages/scope-{}", i),
                language: contract::Language::Unknown("?".into()),
                framework: String::new(),
                build_tool: contract::BuildTool::Unknown("?".into()),
                registry: contract::Registry::None,
                release: contract::StageRelease::default(),
                test_threshold: None,
                ci_workflow: None,
            });
        }
        scopes.push(contract::Scope {
            name: "cli".into(),
            dir: "packages/cli".into(),
            language: contract::Language::Rust,
            framework: String::new(),
            build_tool: contract::BuildTool::Cargo,
            registry: contract::Registry::Crates,
            release: contract::StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        });
        let start = Instant::now();
        let filtered: Vec<_> = scopes
            .iter()
            .filter(|s| {
                let scope_abs = repo_path.join(&s.dir);
                cwd.starts_with(&scope_abs) || scope_abs.starts_with(&cwd)
            })
            .collect();
        let elapsed = start.elapsed();
        assert_eq!(filtered.len(), 1, "应只匹配一个");
        assert_eq!(filtered[0].name, "cli");
        assert!(
            elapsed.as_micros() < 5000,
            "1000 scope 过滤应 < 5ms，实际: {}μs",
            elapsed.as_micros()
        );
    }

    // ── status_to ──────────────────────────────────────────────

    #[test]
    fn test_status_to_passing() {
        let d = tempfile::tempdir().unwrap();
        // 创建一个真实的 Rust 项目，使得 cargo test 能运行并通过
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/lib.rs"), "#[test]\nfn it_works() {}\n").unwrap();

        let c = contract::Contract::default();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path(), &c).unwrap();
        let out = String::from_utf8_lossy(&buf);

        assert!(out.contains("测试状态"));
    }

    #[test]
    fn test_status_to_empty() {
        let d = tempfile::tempdir().unwrap();
        let c = contract::Contract::default();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path(), &c).unwrap();
        let out = String::from_utf8_lossy(&buf);

        assert!(out.contains("测试状态"));
    }
}
