use std::collections::HashSet;

use super::{DependencyRule, DependencyRules};
use cargo_metadata::PackageId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RulesFileSchema {
    rules: Option<RulesSchema>,
}
impl From<RulesFileSchema> for DependencyRules {
    fn from(rules_file: RulesFileSchema) -> Self {
        if let Some(rules) = rules_file.rules {
            let dependency_rules = rules
                .rule
                .iter()
                .map(|rule| {
                    let package = PackageId {
                        repr: rule.package.clone(),
                    };
                    let forbidden_dependencies = HashSet::from_iter(
                        rule.forbidden_dependencies
                            .iter()
                            .map(|p| PackageId { repr: p.clone() }),
                    );

                    DependencyRule::new(package, forbidden_dependencies)
                })
                .collect();
            Self {
                rules: dependency_rules,
            }
        } else {
            Self { rules: Vec::new() }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct RulesSchema {
    rule: Vec<RuleSchema>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct RuleSchema {
    package: String,
    forbidden_dependencies: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use string_auto_indent::auto_indent;

    #[test]
    fn test_into_rules_file_schema_to_dependency_rules() {
        let rules_file = RulesFileSchema {
            rules: Some(RulesSchema {
                rule: vec![RuleSchema {
                    package: "package1".to_string(),
                    forbidden_dependencies: vec!["package2".to_string(), "package3".to_string()],
                }],
            }),
        };
        let expected = DependencyRules {
            rules: vec![DependencyRule::new(
                PackageId {
                    repr: "package1".to_string(),
                },
                HashSet::from([
                    PackageId {
                        repr: "package2".to_string(),
                    },
                    PackageId {
                        repr: "package3".to_string(),
                    },
                ]),
            )],
        };

        let dependency_rules: DependencyRules = rules_file.into();
        assert_eq!(dependency_rules, expected);
    }

    #[test]
    fn test_from_rules_file_schema_to_dependency_rules() {
        let rules_file = RulesFileSchema {
            rules: Some(RulesSchema {
                rule: vec![RuleSchema {
                    package: "package1".to_string(),
                    forbidden_dependencies: vec!["package2".to_string(), "package3".to_string()],
                }],
            }),
        };
        let expected = DependencyRules {
            rules: vec![DependencyRule::new(
                PackageId {
                    repr: "package1".to_string(),
                },
                HashSet::from([
                    PackageId {
                        repr: "package2".to_string(),
                    },
                    PackageId {
                        repr: "package3".to_string(),
                    },
                ]),
            )],
        };

        let dependency_rules = DependencyRules::from(rules_file);
        assert_eq!(dependency_rules, expected);
    }

    #[test]
    fn test_parse_rules_text_inline_table() {
        let rules_text = r#"
            [rules]
            rule = [
                {package = "package1", forbidden_dependencies = ["package2", "package3"]},
            ]
            "#;
        let expected = RulesFileSchema {
            rules: Some(RulesSchema {
                rule: vec![RuleSchema {
                    package: "package1".to_string(),
                    forbidden_dependencies: vec!["package2".to_string(), "package3".to_string()],
                }],
            }),
        };

        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        assert_eq!(rules, expected);
    }

    #[test]
    fn test_parse_rules_text_table() {
        let rules_text = r#"
            [[rules.rule]]
            package = "package1"
            forbidden_dependencies = ["package2", "package3"]

            [[rules.rule]]
            package = "package2"
            forbidden_dependencies = ["package1"]
            "#;
        let expected = RulesFileSchema {
            rules: Some(RulesSchema {
                rule: vec![
                    RuleSchema {
                        package: "package1".to_string(),
                        forbidden_dependencies: vec![
                            "package2".to_string(),
                            "package3".to_string(),
                        ],
                    },
                    RuleSchema {
                        package: "package2".to_string(),
                        forbidden_dependencies: vec!["package1".to_string()],
                    },
                ],
            }),
        };

        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        assert_eq!(rules, expected);
    }

    #[test]
    fn test_serialize_rules_schema() {
        let rules = RulesFileSchema {
            rules: Some(RulesSchema {
                rule: vec![
                    RuleSchema {
                        package: "package1".to_string(),
                        forbidden_dependencies: vec![
                            "package2".to_string(),
                            "package3".to_string(),
                        ],
                    },
                    RuleSchema {
                        package: "package2".to_string(),
                        forbidden_dependencies: vec!["package1".to_string()],
                    },
                ],
            }),
        };
        let expected = r#"
            [[rules.rule]]
            package = "package1"
            forbidden_dependencies = ["package2", "package3"]

            [[rules.rule]]
            package = "package2"
            forbidden_dependencies = ["package1"]
            "#;

        let rules_text = toml::to_string(&rules).unwrap();
        assert_eq!(auto_indent(expected).trim(), rules_text.trim(),);
    }
}
