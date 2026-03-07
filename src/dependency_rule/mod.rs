use anyhow::{Context, Error};
use std::{collections::HashSet, fs};
mod rules_parser;

use rules_parser::RulesFileSchema;

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyRules {
    pub(crate) rules: Vec<DependencyRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DependencyRule {
    pub(crate) package: String,
    pub(crate) forbidden_dependencies: HashSet<String>,
}
impl DependencyRule {
    pub(crate) fn new(package: String, forbidden_dependencies: HashSet<String>) -> Self {
        Self {
            package,
            forbidden_dependencies,
        }
    }
}

impl DependencyRules {
    #[tracing::instrument(skip_all, fields(path = ?path.as_ref()))]
    pub(crate) fn from_file<P>(path: P) -> Result<DependencyRules, Error>
    where
        P: AsRef<std::path::Path>,
    {
        let path = path.as_ref();
        let rules_text: String = fs::read_to_string(path).with_context(|| {
            format!("failed to read dependency rules from '{}'", path.display())
        })?;
        let rules: RulesFileSchema = toml::from_str(&rules_text)
            .with_context(|| format!("failed to parse dependency rules in '{}'", path.display()))?;

        rules
            .try_into()
            .with_context(|| format!("invalid dependency rules in '{}'", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_dependency_rule_from_file() {
        let path = "tests/test_files/parse_rules_test.toml";
        let expected = DependencyRules {
            rules: vec![DependencyRule {
                package: "package1".to_string(),
                forbidden_dependencies: HashSet::from([
                    "package2".to_string(),
                    "package3".to_string(),
                ]),
            }],
        };

        let actual = DependencyRules::from_file(path).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_from_file_nonexistent_path() {
        let result = DependencyRules::from_file("nonexistent/path/rules.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_invalid_toml() {
        let dir = std::env::temp_dir().join("check_deprule_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("invalid.toml");
        std::fs::write(&path, "not valid toml {{{").unwrap();

        let result = DependencyRules::from_file(&path);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_from_file_empty_toml() {
        let dir = std::env::temp_dir().join("check_deprule_test_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.toml");
        std::fs::write(&path, "").unwrap();

        let rules = DependencyRules::from_file(&path).unwrap();
        assert!(rules.rules.is_empty());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
