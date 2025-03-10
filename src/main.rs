use anyhow::{Ok, Result};
use check_deprule::{
    dependency_graph::{build_dependency_graph, DependencyGraphBuildConfigs},
    metadata::{collect_metadata, CollectMetadataConfig},
};

fn main() -> Result<()> {
    let build_config = DependencyGraphBuildConfigs::default();
    let collect_metadata_config = CollectMetadataConfig{
        manifest_path: Some("C:/Users/matsu/Documents/GitHub/check-deprule/tests/init_crate/Cargo.toml".to_string()),
        ..CollectMetadataConfig::default()
    };

    let metadata = collect_metadata(collect_metadata_config)?;
    let graph = build_dependency_graph(metadata, build_config)?;

    println!("{:?}", graph);

    Ok(())
}
