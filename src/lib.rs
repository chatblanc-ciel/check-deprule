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

pub struct HandlerConfig {
    pub graph_build_configs: dependency_graph::DependencyGraphBuildConfigs,
    pub metadata_configs: metadata::CollectMetadataConfig,
    pub tree_config: dependency_graph::tree::TreePrintConfig,
    pub rules_path: Option<PathBuf>,
}

pub fn handler(config: HandlerConfig) -> anyhow::Result<ReturnStatus> {
    let metadata = metadata::collect_metadata(config.metadata_configs.clone())?;
    let graph =
        dependency_graph::build_dependency_graph(metadata.clone(), config.graph_build_configs)?;

    let rules_path = match config.rules_path {
        Some(path) => path,
        None => {
            let manifest_path = match config.metadata_configs.manifest_path {
                Some(path) => PathBuf::from(path),
                None => env::current_dir()?.join("Cargo.toml"),
            };
            let rules_dir = manifest_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("manifest path has no parent directory"))?;
            rules_dir.join("dependency_rules.toml")
        }
    };
    let rules = dependency_rule::DependencyRules::from_file(rules_path)?;

    dependency_graph::tree::print(
        &mut std::io::stdout(),
        &graph,
        &metadata,
        rules,
        config.tree_config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dependency_graph::{DependencyGraphBuildConfigs, tree::TreePrintConfig},
        metadata::CollectMetadataConfig,
    };
    use anyhow::Result;

    fn handler_config(manifest_path: &str) -> HandlerConfig {
        HandlerConfig {
            graph_build_configs: DependencyGraphBuildConfigs::default(),
            metadata_configs: CollectMetadataConfig {
                manifest_path: Some(manifest_path.to_string()),
                ..CollectMetadataConfig::default()
            },
            tree_config: TreePrintConfig::default(),
            rules_path: None,
        }
    }

    #[test]
    #[ignore]
    fn test_main() -> Result<()> {
        let config = handler_config("tests/demo_crates/clean-arch/Cargo.toml");
        let _ = handler(config)?;
        Ok(())
    }

    #[test]
    fn test_handler_success() -> Result<()> {
        let config = handler_config("tests/demo_crates/clean-arch/Cargo.toml");
        let result = handler(config)?;
        assert_eq!(result.to_return_code(), ExitCode::SUCCESS);
        Ok(())
    }

    #[test]
    fn test_handler_failure() -> Result<()> {
        let config = handler_config("tests/demo_crates/tangled-clean-arch/Cargo.toml");
        let result = handler(config)?;
        assert_eq!(result.to_return_code(), ExitCode::FAILURE);
        Ok(())
    }
}
