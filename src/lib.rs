use std::{env, path::PathBuf, process::ExitCode};

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

    pub fn merge(self, has_violation: bool) -> Self {
        if has_violation || matches!(self, Self::Violation) {
            Self::Violation
        } else {
            Self::NoViolation
        }
    }
}

pub fn handler(
    graph_build_configs: dependency_graph::DependencyGraphBuildConfigs,
    metadata_configs: metadata::CollectMetadataConfig,
) -> anyhow::Result<ReturnStatus> {
    let metadata = metadata::collect_metadata(metadata_configs.clone())?;
    let graph = dependency_graph::build_dependency_graph(metadata.clone(), graph_build_configs)?;

    let manifest_path = match metadata_configs.manifest_path {
        Some(path) => PathBuf::from(path),
        None => env::current_dir()?.join("Cargo.toml"),
    };
    let rules_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("manifest path has no parent directory"))?;
    let rules =
        dependency_rule::DependencyRules::from_file(rules_dir.join("dependency_rules.toml"))?;

    dependency_graph::tree::print(
        &mut std::io::stdout(),
        &graph,
        &metadata,
        rules,
        dependency_graph::tree::TreePrintConfig::default(),
    )
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
