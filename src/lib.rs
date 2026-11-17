use anyhow::Context;
use std::{env, io::Write, path::PathBuf, process::ExitCode};

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

pub struct HandlerConfig {
    pub graph_build_configs: dependency_graph::DependencyGraphBuildConfigs,
    pub metadata_configs: metadata::CollectMetadataConfig,
    pub tree_config: dependency_graph::tree::TreePrintConfig,
    pub print_tree: bool,
    pub rules_path: Option<PathBuf>,
}

pub fn handler(config: HandlerConfig) -> anyhow::Result<ReturnStatus> {
    handle(config, &mut std::io::stdout().lock())
}

fn handle(config: HandlerConfig, writer: &mut impl Write) -> anyhow::Result<ReturnStatus> {
    tracing::info!("collecting cargo metadata");
    let metadata = metadata::collect_metadata(config.metadata_configs.clone())?;

    tracing::info!("building dependency graph");
    let graph = dependency_graph::build_dependency_graph(&metadata, config.graph_build_configs)?;

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
    tracing::info!(path = ?rules_path, "loading dependency rules");
    let rules = dependency_rule::DependencyRules::from_file(&rules_path).with_context(|| {
        format!(
            "failed to load dependency rules from '{}'",
            rules_path.display()
        )
    })?;

    tracing::info!("checking violations");
    let report = dependency_graph::violation::check_violations(&graph, &rules);

    if config.print_tree {
        tracing::info!("printing dependency tree");
        dependency_graph::tree::print(writer, &graph, &metadata, &report, config.tree_config)?;
    }

    tracing::info!("printing violations");
    dependency_graph::violation::print(writer, &report)?;

    if report.has_violations() {
        Ok(ReturnStatus::Violation)
    } else {
        Ok(ReturnStatus::NoViolation)
    }
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
            print_tree: false,
            rules_path: None,
        }
    }

    fn handle_to_string(config: HandlerConfig) -> Result<(ReturnStatus, String)> {
        let mut buf = Vec::new();
        let status = handle(config, &mut buf)?;
        Ok((status, String::from_utf8(buf)?))
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

    #[test]
    fn test_handler_output_no_violation() -> Result<()> {
        let config = handler_config("tests/demo_crates/clean-arch/Cargo.toml");
        let (status, output) = handle_to_string(config)?;

        assert_eq!(status.to_return_code(), ExitCode::SUCCESS);
        assert_eq!(output, "no violations found\n");
        Ok(())
    }

    #[test]
    fn test_handler_output_lists_violations_without_tree() -> Result<()> {
        let config = handler_config("tests/demo_crates/tangled-clean-arch/Cargo.toml");
        let (status, output) = handle_to_string(config)?;

        assert_eq!(status.to_return_code(), ExitCode::FAILURE);
        assert!(output.contains("violation: tangled-ca-core -> tangled-ca-handler\n"));
        assert!(output.ends_with("5 violations found\n"));
        assert!(
            output
                .lines()
                .all(|line| line.starts_with("violation: ") || line == "5 violations found"),
            "default output must not include the dependency tree: {output}"
        );
        Ok(())
    }

    #[test]
    fn test_handler_output_tree() -> Result<()> {
        let config = HandlerConfig {
            print_tree: true,
            ..handler_config("tests/demo_crates/tangled-clean-arch/Cargo.toml")
        };
        let (status, output) = handle_to_string(config)?;

        assert_eq!(status.to_return_code(), ExitCode::FAILURE);
        assert!(output.contains("└── "), "tree should be printed: {output}");
        assert!(output.lines().any(|line| line.ends_with(" [violation]")));
        assert!(output.ends_with("5 violations found\n"));
        Ok(())
    }
}
