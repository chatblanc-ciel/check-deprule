use std::collections::HashSet;

use super::{DependencyRule, DependencyRules};
use anyhow::{Error, bail};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RulesFileSchema {
    rules: Option<RulesSchema>,
}

impl TryFrom<RulesFileSchema> for DependencyRules {
    type Error = Error;

    fn try_from(rules_file: RulesFileSchema) -> Result<Self, Self::Error> {
        let Some(rules) = rules_file.rules else {
            return Ok(Self { rules: Vec::new() });
        };

        validate_rules(&rules.rule)?;

        let dependency_rules = rules
            .rule
            .into_iter()
            .map(|rule| {
                let forbidden_dependencies = HashSet::from_iter(rule.forbidden_dependencies);
                DependencyRule::new(rule.package, forbidden_dependencies)
            })
            .collect();

        Ok(Self {
            rules: dependency_rules,
        })
    }
}

fn validate_rules(rules: &[RuleSchema]) -> Result<(), Error> {
    let mut seen_packages = HashSet::new();

    for rule in rules {
        if rule.package.is_empty() {
            bail!("rule has an empty package name");
        }

        if !seen_packages.insert(&rule.package) {
            bail!("duplicate rule definition for package '{}'", rule.package);
        }

        for dep in &rule.forbidden_dependencies {
            if dep.is_empty() {
                bail!(
                    "rule for package '{}': forbidden_dependency is empty",
                    rule.package
                );
            }
        }

        if rule.forbidden_dependencies.contains(&rule.package) {
            bail!(
                "rule for package '{}': package cannot forbid itself",
                rule.package
            );
        }
    }

    Ok(())
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
    fn test_try_from_rules_file_schema_to_dependency_rules() {
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
                "package1".to_string(),
                HashSet::from(["package2".to_string(), "package3".to_string()]),
            )],
        };

        let dependency_rules = DependencyRules::try_from(rules_file).unwrap();
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

    #[test]
    fn test_parse_empty_toml() {
        let rules_text = "";
        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        let dependency_rules = DependencyRules::try_from(rules).unwrap();
        assert!(dependency_rules.rules.is_empty());
    }

    #[test]
    fn test_parse_rules_section_without_rule() {
        let rules_text = "[rules]";
        let result: Result<RulesFileSchema, _> = toml::from_str(rules_text);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_forbidden_dependencies() {
        let rules_text = r#"
            [[rules.rule]]
            package = "package1"
            forbidden_dependencies = []
            "#;
        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        let dependency_rules = DependencyRules::try_from(rules).unwrap();
        assert_eq!(dependency_rules.rules.len(), 1);
        assert!(dependency_rules.rules[0].forbidden_dependencies.is_empty());
    }

    #[test]
    fn test_parse_invalid_toml_syntax() {
        let rules_text = "this is not valid toml {{{";
        let result: Result<RulesFileSchema, _> = toml::from_str(rules_text);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_package_name() {
        let rules_text = r#"
            [[rules.rule]]
            package = ""
            forbidden_dependencies = ["package2"]
            "#;
        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        let result = DependencyRules::try_from(rules);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("empty package name")
        );
    }

    #[test]
    fn test_validate_empty_forbidden_dependency_name() {
        let rules_text = r#"
            [[rules.rule]]
            package = "package1"
            forbidden_dependencies = ["package2", ""]
            "#;
        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        let result = DependencyRules::try_from(rules);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("forbidden_dependency is empty")
        );
    }

    #[test]
    fn test_validate_duplicate_package_rules() {
        let rules_text = r#"
            [[rules.rule]]
            package = "package1"
            forbidden_dependencies = ["package2"]

            [[rules.rule]]
            package = "package1"
            forbidden_dependencies = ["package3"]
            "#;
        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        let result = DependencyRules::try_from(rules);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("duplicate rule definition for package 'package1'")
        );
    }

    #[test]
    fn test_validate_self_reference() {
        let rules_text = r#"
            [[rules.rule]]
            package = "package1"
            forbidden_dependencies = ["package1"]
            "#;
        let rules: RulesFileSchema = toml::from_str(rules_text).unwrap();
        let result = DependencyRules::try_from(rules);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot forbid itself")
        );
    }
}
