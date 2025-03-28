use anyhow::{Error, Ok};
use cargo_metadata::PackageId;
use std::{collections::HashSet, fs};
mod rules_parser;

use rules_parser::RulesFileSchema;

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyRules {
    pub(crate) rules: Vec<DependencyRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DependencyRule {
    pub(crate) package: PackageId,
    pub(crate) forbidden_dependencies: HashSet<PackageId>,
}
impl DependencyRule {
    pub(crate) fn new(package: PackageId, forbidden_dependencies: HashSet<PackageId>) -> Self {
        Self {
            package,
            forbidden_dependencies,
        }
    }

    pub(crate) fn from_file<P>(path: P) -> Result<DependencyRules, Error>
    where
        P: AsRef<std::path::Path>,
    {
        let rules_text: String = fs::read_to_string(path)?;
        let rules: RulesFileSchema = toml::from_str(&rules_text)?;

        Ok(rules.into())
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
                package: PackageId {
                    repr: "package1".to_string(),
                },
                forbidden_dependencies: HashSet::from_iter([
                    PackageId {
                        repr: "package2".to_string(),
                    },
                    PackageId {
                        repr: "package3".to_string(),
                    },
                ]),
            }],
        };

        let actual = DependencyRule::from_file(path).unwrap();
        assert_eq!(expected, actual);
    }
}
