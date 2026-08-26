pub use quanttide_devops::source::config_file::{detect_languages, read_config_versions};

use crate::contract::Language;
use std::path::Path;

/// 按目录中的文件推测语言。
pub fn detect_by_files(dir: &Path) -> Language {
    detect_languages(dir)
        .into_iter()
        .next()
        .unwrap_or(Language::Unknown(String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_detect_by_files_empty_dir() {
        let d = tmpdir();
        let lang = detect_by_files(d.path());
        assert!(matches!(lang, Language::Unknown(_)));
    }

    #[test]
    fn test_detect_by_files_cargo_toml() {
        let d = tmpdir();
        std::fs::write(d.path().join("Cargo.toml"), "").unwrap();
        let lang = detect_by_files(d.path());
        assert_eq!(lang.as_str(), "rust");
    }

    #[test]
    fn test_detect_by_files_python() {
        let d = tmpdir();
        std::fs::write(d.path().join("pyproject.toml"), "").unwrap();
        let lang = detect_by_files(d.path());
        assert_eq!(lang.as_str(), "python");
    }
}
