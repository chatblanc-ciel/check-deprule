use anyhow::{Ok, Result};
use check_deprule::{
    dependency_graph::{DependencyGraphBuildConfigs, build_dependency_graph},
    metadata::{CollectMetadataConfig, collect_metadata},
};

fn main() -> Result<()> {
    let build_config = DependencyGraphBuildConfigs::default();
    let collect_metadata_config = CollectMetadataConfig {
        manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
        ..CollectMetadataConfig::default()
    };

    let metadata = collect_metadata(collect_metadata_config)?;
    let graph = build_dependency_graph(metadata, build_config)?;

    check_deprule::dependency_graph::tree::print(&graph)?;

    Ok(())
}
