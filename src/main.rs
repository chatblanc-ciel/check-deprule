use std::process::ExitCode;

use anyhow::{Ok, Result};
use check_deprule::{
    dependency_graph::DependencyGraphBuildConfigs, handler, metadata::CollectMetadataConfig,
};

fn main() -> Result<ExitCode> {
    let build_config = DependencyGraphBuildConfigs::default();
    let collect_metadata_config = CollectMetadataConfig {
        manifest_path: None,
        ..CollectMetadataConfig::default()
    };

    let result = handler(build_config, collect_metadata_config)?;

    Ok(result.to_return_code())
}
