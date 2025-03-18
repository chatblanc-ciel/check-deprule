use std::{env, path::Path, process::ExitCode};

pub mod dependency_graph;
pub mod dependency_rule;
pub mod metadata;

#[derive(Debug, Clone)]
pub enum ReturnStatus {
    NoViolation,
    Violation,
}
impl ReturnStatus {
    pub fn to_return_code(&self) -> ExitCode {
        match self {
            ReturnStatus::NoViolation => ExitCode::SUCCESS,
            ReturnStatus::Violation => ExitCode::FAILURE,
        }
    }
}

pub fn handler(
    graph_build_configs: dependency_graph::DependencyGraphBuildConfigs,
    metadata_configs: metadata::CollectMetadataConfig,
) -> anyhow::Result<ReturnStatus> {
    let metadata = metadata::collect_metadata(metadata_configs.clone())?;
    let graph = dependency_graph::build_dependency_graph(metadata, graph_build_configs)?;

    if let Some(manifest_path) = metadata_configs.manifest_path {
        let manifest_path = Path::new(&manifest_path);
        let rules = dependency_rule::DependencyRule::from_file(
            manifest_path
                .parent()
                .unwrap()
                .join(Path::new("dependency_rules.toml")),
        )?;

        dependency_graph::tree::print(&graph, manifest_path, rules)
    } else {
        let current_dir = env::current_dir()?;
        let manifest_path = current_dir.join(Path::new("Cargo.toml"));
        let rules = dependency_rule::DependencyRule::from_file(
            manifest_path
                .parent()
                .unwrap()
                .join(Path::new("dependency_rules.toml")),
        )?;

        dependency_graph::tree::print(&graph, manifest_path, rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dependency_graph::DependencyGraphBuildConfigs, metadata::CollectMetadataConfig};
    use anyhow::Result;

    #[test]
    #[ignore]
    fn test_main() -> Result<()> {
        let build_config = DependencyGraphBuildConfigs::default();
        let collect_metadata_config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };

        let _ = handler(build_config, collect_metadata_config)?;

        Ok(())
    }

    #[test]
    fn test_handler_success() -> Result<()> {
        let expected_return_code = ExitCode::SUCCESS;
        let build_config = DependencyGraphBuildConfigs::default();
        let collect_metadata_config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };

        let result = handler(build_config, collect_metadata_config)?;
        assert_eq!(result.to_return_code(), expected_return_code);

        Ok(())
    }

    #[test]
    fn test_handler_failure() -> Result<()> {
        let expected_return_code = ExitCode::FAILURE;
        let build_config = DependencyGraphBuildConfigs::default();
        let collect_metadata_config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/tangled-clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };

        let result = handler(build_config, collect_metadata_config)?;
        assert_eq!(result.to_return_code(), expected_return_code);

        Ok(())
    }
}
