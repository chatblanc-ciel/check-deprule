use super::Graph;
use crate::dependency_rule::DependencyRules;
use anyhow::Error;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::collections::HashSet;
use std::io::Write;

#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub parent: String,
    pub dependency: String,
}

#[derive(Debug, Clone, Default)]
pub struct ViolationReport {
    pub violations: Vec<Violation>,
    violated_edges: HashSet<(String, String)>,
}

impl ViolationReport {
    pub fn is_violation(&self, parent_name: &str, dependency_name: &str) -> bool {
        self.violated_edges
            .contains(&(parent_name.to_string(), dependency_name.to_string()))
    }

    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }
}

#[tracing::instrument(skip_all)]
pub fn check_violations(graph: &Graph, rules: &DependencyRules) -> ViolationReport {
    let mut violations = Vec::new();
    let mut violated_edges = HashSet::new();

    for edge in graph.graph.edge_references() {
        let parent = &graph.graph[edge.source()];
        let child = &graph.graph[edge.target()];

        let is_forbidden = rules.rules.iter().any(|rule| {
            rule.package == parent.name && rule.forbidden_dependencies.contains(&child.name)
        });

        if is_forbidden && violated_edges.insert((parent.name.clone(), child.name.clone())) {
            violations.push(Violation {
                parent: parent.name.clone(),
                dependency: child.name.clone(),
            });
        }
    }

    ViolationReport {
        violations,
        violated_edges,
    }
}

pub fn print(writer: &mut impl Write, report: &ViolationReport) -> Result<(), Error> {
    let mut violations: Vec<&Violation> = report.violations.iter().collect();
    violations.sort_by(|a, b| {
        a.parent
            .cmp(&b.parent)
            .then_with(|| a.dependency.cmp(&b.dependency))
    });

    for violation in &violations {
        writeln!(
            writer,
            "violation: {} -> {}",
            violation.parent, violation.dependency
        )?;
    }

    match violations.len() {
        0 => writeln!(writer, "no violations found")?,
        1 => writeln!(writer, "1 violation found")?,
        n => writeln!(writer, "{n} violations found")?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_graph::{DependencyGraphBuildConfigs, build_dependency_graph};
    use crate::dependency_rule::DependencyRules;
    use crate::metadata::{CollectMetadataConfig, collect_metadata};
    use anyhow::Result;

    #[test]
    fn test_check_violations_no_violation() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph = build_dependency_graph(&metadata, DependencyGraphBuildConfigs::default())?;
        let rules =
            DependencyRules::from_file("tests/demo_crates/clean-arch/dependency_rules.toml")?;

        let report = check_violations(&graph, &rules);

        assert!(!report.has_violations());
        assert!(report.violations.is_empty());
        Ok(())
    }

    #[test]
    fn test_check_violations_with_violation() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/tangled-clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph = build_dependency_graph(&metadata, DependencyGraphBuildConfigs::default())?;
        let rules = DependencyRules::from_file(
            "tests/demo_crates/tangled-clean-arch/dependency_rules.toml",
        )?;

        let report = check_violations(&graph, &rules);

        assert!(report.has_violations());
        assert!(!report.violations.is_empty());
        Ok(())
    }

    #[test]
    fn test_check_violations_is_violation_lookup() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/tangled-clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph = build_dependency_graph(&metadata, DependencyGraphBuildConfigs::default())?;
        let rules = DependencyRules::from_file(
            "tests/demo_crates/tangled-clean-arch/dependency_rules.toml",
        )?;

        let report = check_violations(&graph, &rules);

        // At least one violation should be detectable via is_violation
        let has_lookup_match = report
            .violations
            .iter()
            .any(|v| report.is_violation(&v.parent, &v.dependency));
        assert!(has_lookup_match);

        // Non-existent edge should not be a violation
        assert!(!report.is_violation("nonexistent-pkg", "another-pkg"));
        Ok(())
    }

    #[test]
    fn test_empty_report_default() {
        let report = ViolationReport::default();
        assert!(!report.has_violations());
        assert!(report.violations.is_empty());
        assert!(!report.is_violation("any", "pkg"));
    }

    #[test]
    fn test_check_violations_with_empty_rules() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/tangled-clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph = build_dependency_graph(&metadata, DependencyGraphBuildConfigs::default())?;
        let rules = DependencyRules { rules: Vec::new() };

        let report = check_violations(&graph, &rules);

        assert!(!report.has_violations());
        assert!(report.violations.is_empty());
        Ok(())
    }

    #[test]
    fn test_check_violations_rule_package_not_in_graph() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph = build_dependency_graph(&metadata, DependencyGraphBuildConfigs::default())?;

        // グラフに存在しないパッケージ名のルール
        let rules = DependencyRules {
            rules: vec![crate::dependency_rule::DependencyRule::new(
                "nonexistent-package".to_string(),
                HashSet::from(["also-nonexistent".to_string()]),
            )],
        };

        let report = check_violations(&graph, &rules);

        assert!(!report.has_violations());
        Ok(())
    }

    #[test]
    fn test_print_violations_lists_each_edge_sorted() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/tangled-clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph = build_dependency_graph(&metadata, DependencyGraphBuildConfigs::default())?;
        let rules = DependencyRules::from_file(
            "tests/demo_crates/tangled-clean-arch/dependency_rules.toml",
        )?;
        let report = check_violations(&graph, &rules);

        let mut buf = Vec::new();
        print(&mut buf, &report)?;

        let expected = "\
violation: tangled-ca-core -> tangled-ca-database
violation: tangled-ca-core -> tangled-ca-handler
violation: tangled-ca-interactor -> tangled-ca-database
violation: tangled-ca-interactor -> tangled-ca-handler
violation: tangled-ca-interactor -> tangled-ca-repositry
5 violations found
";
        assert_eq!(String::from_utf8(buf)?, expected);
        Ok(())
    }

    #[test]
    fn test_print_violations_single() -> Result<()> {
        let report = ViolationReport {
            violations: vec![Violation {
                parent: "core".to_string(),
                dependency: "handler".to_string(),
            }],
            violated_edges: HashSet::from([("core".to_string(), "handler".to_string())]),
        };

        let mut buf = Vec::new();
        print(&mut buf, &report)?;

        assert_eq!(
            String::from_utf8(buf)?,
            "violation: core -> handler\n1 violation found\n"
        );
        Ok(())
    }

    #[test]
    fn test_print_violations_no_violation() -> Result<()> {
        let mut buf = Vec::new();
        print(&mut buf, &ViolationReport::default())?;

        assert_eq!(String::from_utf8(buf)?, "no violations found\n");
        Ok(())
    }
}
