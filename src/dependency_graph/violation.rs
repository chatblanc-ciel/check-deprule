use super::Graph;
use crate::dependency_rule::DependencyRules;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::collections::HashSet;

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
}
