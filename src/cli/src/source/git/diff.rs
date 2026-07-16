use std::path::Path;

/// 获取自最新 tag 以来的变更文件列表。
pub fn get_changed_paths_since_last_tag(root: &Path) -> Vec<String> {
    let tags = crate::source::git::tag::collect_tags_with_scope(root);
    let latest_tag = tags
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .find_map(|(_, v)| v.first())
        .or_else(|| tags.get("(root)").and_then(|v| v.first()));

    let range = match latest_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => return vec![],
    };

    let output = crate::source::git::git(&["diff", "--name-only", &range], root).unwrap_or_default();
    output.lines().map(|s| s.to_string()).collect()
}
