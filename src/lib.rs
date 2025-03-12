use std::{env, path::Path};

pub mod dependency_graph;
pub mod dependency_rule;
pub mod metadata;

pub fn handler(
    graph_build_configs: dependency_graph::DependencyGraphBuildConfigs,
    metadata_configs: metadata::CollectMetadataConfig,
) -> anyhow::Result<()> {
    let metadata = metadata::collect_metadata(metadata_configs.clone())?;
    let graph = dependency_graph::build_dependency_graph(metadata, graph_build_configs)?;

    if let Some(manifest_path) = metadata_configs.manifest_path {
        dependency_graph::tree::print(&graph, Path::new(&manifest_path))?;
    } else {
        let manifest_path = format!("{}/Cargo.toml", env::current_dir()?.display());
        dependency_graph::tree::print(&graph, manifest_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dependency_graph::DependencyGraphBuildConfigs, metadata::CollectMetadataConfig};
    use anyhow::Result;

    #[test]
    fn test_main() -> Result<()> {
        let build_config = DependencyGraphBuildConfigs::default();
        let collect_metadata_config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };

        handler(build_config, collect_metadata_config)?;

        Ok(())
    }
}
