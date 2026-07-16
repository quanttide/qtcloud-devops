
#[cfg(test)]
mod tests {
    use crate::plan::*;
    use crate::plan::clean::*;
    use crate::plan::audit::*;
    use std::io::Write;
    use std::path::Path;

    fn write_roadmap(content: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(d.path().join("ROADMAP.md")).unwrap();
        write!(f, "{}", content).unwrap();
        d
    }

    fn read_roadmap(d: &Path) -> String {
        std::fs::read_to_string(d.join("ROADMAP.md")).unwrap_or_default()
    }

    // ── parse_roadmap ────────────────────────────────────────────

    #[test]
    fn test_parse_empty() {
        let d = write_roadmap("");
        let result = parse_roadmap(&d.path().join("ROADMAP.md"));
        assert!(result.is_err(), "空文件应解析失败");
    }

    #[test]
    fn test_parse_single_version() {
        let d = write_roadmap(
            "# ROADMAP\n\
             ## [0.1.0]\n\
             \n\
             ### Added\n\
             - [x] feature a\n\
             - [ ] feature b\n\
             ### Fixed\n\
             - [x] bug c\n",
        );
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].version, "0.1.0");
        assert_eq!(v[0].done, 2);
        assert_eq!(v[0].total, 3);
    }

    #[test]
    fn test_parse_multi_version() {
        let d = write_roadmap(
            "# ROADMAP\n\
             ## [0.2.0]\n\
             ### Added\n\
             - [x] done\n\
             - [ ] todo\n\
             \n\
             ## [0.1.0]\n\
             ### Added\n\
             - [x] a\n\
             - [x] b\n",
        );
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].version, "0.2.0");
        assert_eq!(v[0].done, 1);
        assert_eq!(v[0].total, 2);
        assert_eq!(v[1].version, "0.1.0");
        assert_eq!(v[1].done, 2);
        assert_eq!(v[1].total, 2);
    }

    #[test]
    fn test_parse_v_prefix() {
        let d = write_roadmap("# ROADMAP\n## [v0.1.0]\n### Added\n- [x] item\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v[0].version, "0.1.0");
    }

    #[test]
    fn test_parse_no_checkboxes() {
        let d = write_roadmap("# ROADMAP\n## [0.1.0]\n\njust text\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].done, 0);
        assert_eq!(v[0].total, 0);
    }

    #[test]
    fn test_parse_version_with_suffix() {
        let d = write_roadmap("# ROADMAP\n## [0.1.0] — 已发布\n### Added\n- [x] done\n- [ ] todo\n");
        let v = parse_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].version, "0.1.0");
        assert_eq!(v[0].done, 1);
        assert_eq!(v[0].total, 2);
    }

    #[test]
    fn test_clean_version_with_suffix() {
        // 后缀版本头应被识别并可级联清理
        let d = write_roadmap("# ROADMAP\n## [0.1.0] — 已发布\n### Added\n- [x] done\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        assert!(!content.contains("0.1.0"), "空版本应被清理");
    }

    #[test]
    fn test_parse_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let result = parse_roadmap(&d.path().join("NONEXISTENT.md"));
        assert!(result.is_err());
    }

    // ── resolve_roadmap_path ────────────────────────────────────

    #[test]
    fn test_resolve_path_with_contract_scope() {
        let d = tempfile::tempdir().unwrap();
        // 创建 scope 契约
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  cli:\n    dir: src/cli\n    language: rust\n",
        )
        .unwrap();
        let path = resolve_roadmap_path(d.path(), Some("cli"));
        assert!(path.to_string_lossy().ends_with("src/cli/ROADMAP.md"));
    }

    #[test]
    fn test_resolve_path_fallback_to_name() {
        let d = tempfile::tempdir().unwrap();
        let path = resolve_roadmap_path(d.path(), Some("custom"));
        // scope 不在契约中 → 回退为子目录名
        assert!(path.to_string_lossy().ends_with("custom/ROADMAP.md"));
    }

    #[test]
    fn test_resolve_path_no_scope_no_contract() {
        let d = tempfile::tempdir().unwrap();
        let path = resolve_roadmap_path(d.path(), None);
        // 无 scope + 无契约 → repo 根目录
        assert_eq!(path, d.path().join("ROADMAP.md"));
    }

    // ── clean_roadmap ───────────────────────────────────────────

    #[test]
    fn test_clean_removes_done_items() {
        let d = write_roadmap(
            "## [0.1.0]\n\
             ### Added\n\
             - [x] done item\n\
             - [ ] todo item\n\
             ### Fixed\n\
             - [x] fixed bug\n",
        );
        let removed = clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert!(removed > 0);
        let content = read_roadmap(d.path());
        assert!(!content.contains("done item"));
        assert!(!content.contains("fixed bug"));
        assert!(content.contains("todo item"));
    }

    #[test]
    fn test_clean_empty_file() {
        let d = write_roadmap("");
        let removed = clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_clean_all_done_empties_file() {
        // 所有条目都是 done → 清理后只剩空文件
        let d = write_roadmap("## [0.1.0]\n### Added\n- [x] done\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        assert!(content.is_empty());
    }

    #[test]
    fn test_clean_no_done_items_no_change() {
        let d = write_roadmap("## [0.1.0]\n- [ ] todo\n");
        let removed = clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_clean_cascade_does_not_delete_adjacent_version() {
        // Issue #5-4: [0.5.0] 全 done 被清后，[0.6.0] 不应被连带删除
        let content = "## [0.6.0]\n\
- [ ] 修复 bug\n\
\n\
## [0.5.0]\n\
- [x] 已删除 legacy\n";
        let d = write_roadmap(content);
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let result = read_roadmap(d.path());
        assert!(result.contains("0.6.0"), "[0.6.0] 不应被删除: {}", result);
        assert!(!result.contains("0.5.0"), "[0.5.0] 应被删除: {}", result);
        assert!(result.contains("修复 bug"), "内容应保留: {}", result);
    }

    #[test]
    fn test_clean_trailing_newlines_removed() {
        // 末尾多余空行应被清理
        let d = write_roadmap("## [0.1.0]\n- [ ] todo\n\n\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        assert_eq!(content.trim_end().lines().count(), 2); // 版本标题 + 条目
    }

    #[test]
    fn test_clean_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let nonexistent = d.path().join("NONEXISTENT.md");
        let result = clean_roadmap(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_clean_suffix_version_all_done_cascade() {
        let d = write_roadmap("## [0.2.0]\n\n- [ ] 待办\n\n## [0.1.0] — 已发布\n\n- [x] 旧功能\n");
        clean_roadmap(&d.path().join("ROADMAP.md")).unwrap();
        let content = read_roadmap(d.path());
        // 0.1.0 版本应被删除（全部 done 且带后缀）
        assert!(!content.contains("0.1.0"), "0.1.0 版本应被删除");
        // 0.2.0 版本应保留
        assert!(content.contains("0.2.0"), "0.2.0 版本应保留");
        // 待办内容应保留
        assert!(content.contains("待办"), "待办内容应保留");
    }

    // ── edit_roadmap ────────────────────────────────────────

    #[test]
    fn test_edit_fixes_v_prefix() {
        let d = write_roadmap("## [v0.1.0]\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(issues.iter().any(|f| f.message.contains("v 前缀")));
        let content = read_roadmap(d.path());
        assert!(!content.contains("## [v"));
    }

    #[test]
    fn test_edit_fixes_category_case() {
        let d = write_roadmap("## [0.1.0]\n### added\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(issues.iter().any(|f| f.message.contains("大小写")));
        let content = read_roadmap(d.path());
        assert!(content.contains("### Added"));
    }

    #[test]
    fn test_edit_clean_file_no_issues() {
        let d = write_roadmap("## [0.1.0]\n### Added\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_edit_modifies_file() {
        let d = write_roadmap("## [v0.1.0]\n### ADDED\n-  [x] bad\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(!issues.is_empty());
        let content = read_roadmap(d.path());
        assert!(content.contains("## [0.1.0]"));
        assert!(content.contains("### Added"));
        assert!(content.contains("- [x] bad"));
    }

    #[test]
    fn test_edit_detects_nonstandard_header() {
        let d = write_roadmap("## 现状 (Current)\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(
            issues.iter().any(|i| i.message.contains("非标准版本头")),
            "应检测到非标准版本头: {:?}",
            issues
        );
    }

    #[test]
    fn test_edit_detects_nonstandard_category() {
        let d = write_roadmap("## [0.1.0]\n### 0.1 fix bug\n- [ ] item\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(
            issues.iter().any(|i| i.message.contains("非标准分类")),
            "应检测到非标准分类: {:?}",
            issues
        );
    }

    #[test]
    fn test_edit_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let nonexistent = d.path().join("NONEXISTENT.md");
        let result = edit_roadmap(&nonexistent, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_mixed_format() {
        let d = write_roadmap("## [0.1.0]\n\n- [ ] 标准条目\n\n## 杂项 (Misc)\n\n- [ ] 非标准\n");
        let issues = edit_roadmap(&d.path().join("ROADMAP.md"), "test").unwrap();
        assert!(
            issues.iter().any(|i| i.message.contains("非标准版本头")),
            "应检测到非标准版本头: {:?}",
            issues
        );
    }

    // ── print_status_to ─────────────────────────────────────────

    #[test]
    fn test_print_status_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("未创建规划文件"));
    }

    #[test]
    fn test_print_status_empty_roadmap() {
        let d = write_roadmap("");
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("未找到标准规划条目"));
    }

    #[test]
    fn test_print_status_unknown_headers_warns() {
        // 非标准 ## 头应触发 warning
        let d = write_roadmap("# ROADMAP\n## 现状 (Current)\n- [ ] item\n");
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(
            output.contains("plan edit"),
            "应提示运行 plan edit: {}",
            output
        );
    }

    #[test]
    fn test_print_status_to_with_scope() {
        // scope "test" 不在契约中 → 回退到 test/ROADMAP.md
        let d = tempfile::tempdir().unwrap();
        let scope_dir = d.path().join("test");
        std::fs::create_dir_all(&scope_dir).unwrap();
        std::fs::write(
            scope_dir.join("ROADMAP.md"),
            "# ROADMAP\n## [0.1.0]\n### Added\n- [x] done\n- [ ] todo\n",
        )
        .unwrap();
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), Some("test")).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("test"), "应显示 scope 名称");
        assert!(out.contains("0.1.0"), "应显示版本号");
    }

    #[test]
    fn test_print_status_with_data() {
        let d =
            write_roadmap("# ROADMAP\n## [0.2.0]\n### Added\n- [x] done\n- [ ] todo\n\n## [0.1.0]\n### Added\n- [x] a\n- [x] b\n");
        let mut buf = Vec::new();
        print_status_to(&mut buf, d.path(), None).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("(auto)"));
        assert!(output.contains("0.2.0"));
        assert!(output.contains("0.1.0"));
        assert!(output.contains("3/4"));
        assert!(output.contains("总计"));
    }

    // ── extract_line_paths ───────────────────────────────────────

    #[test]
    fn test_extract_line_paths_simple() {
        let paths = extract_line_paths("- [ ] `src/main.rs` `run_plan_clean`：重构");
        assert_eq!(paths, vec!["src/main.rs"]);
    }

    #[test]
    fn test_extract_line_paths_multiple() {
        let paths = extract_line_paths("- [ ] `src/plan.rs` `plan_audit`：新增路径检查");
        assert_eq!(paths, vec!["src/plan.rs"]);
    }

    #[test]
    fn test_extract_line_paths_no_path_returns_empty() {
        let paths = extract_line_paths("- [ ] 修复登录页面样式");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_extract_line_paths_skip_non_path_backtick() {
        let paths = extract_line_paths("- [ ] `clean` 命令支持 `--all` 参数");
        assert!(paths.is_empty());
    }

    #[test]
    fn test_extract_line_paths_strips_line_number() {
        let paths = extract_line_paths("- [ ] `src/foo.rs:123` 修复 bug");
        assert_eq!(paths, vec!["src/foo.rs"]);
    }

    #[test]
    fn test_extract_line_paths_with_colon_no_number() {
        let paths = extract_line_paths("- [ ] `docs/README.md` 更新文档");
        assert_eq!(paths, vec!["docs/README.md"]);
    }

    // ── plan_audit path checks ───────────────────────────────────

    fn write_todo(content: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("TODO.md"), content).unwrap();
        d
    }

    #[test]
    fn test_audit_path_missing() {
        let d = write_todo("- [ ] `nonexistent/module.rs`：待实现\n");
        // 创建空 ROADMAP.md 避免提前退出
        let _ = std::fs::write(d.path().join("ROADMAP.md"), "");
        let result = plan_audit(d.path());
        assert!(result.is_err(), "路径不存在应使审计失败");
    }

    #[test]
    fn test_audit_granularity_warn() {
        let d = write_todo("- [ ] 缺少文件路径的条目\n");
        let _ = std::fs::write(d.path().join("ROADMAP.md"), "");
        let result = plan_audit(d.path());
        assert!(result.is_err(), "无路径条目应使审计失败");
    }

    #[test]
    fn test_audit_path_exists() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/main.rs"), "").unwrap();
        std::fs::write(
            d.path().join("TODO.md"),
            "- [ ] `src/main.rs` 实现功能\n",
        )
        .unwrap();
        let _ = std::fs::write(d.path().join("ROADMAP.md"), "");
        let result = plan_audit(d.path());
        assert!(result.is_ok(), "路径存在应通过审计: {:?}", result);
    }
}
