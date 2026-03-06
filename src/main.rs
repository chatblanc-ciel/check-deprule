use std::{path::PathBuf, process::ExitCode};

use anyhow::{Ok, Result};
use check_deprule::{
    HandlerConfig,
    dependency_graph::{
        DependencyGraphBuildConfigs,
        tree::{Charset, Prefix, TreePrintConfig},
    },
    handler,
    metadata::CollectMetadataConfig,
};
use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(
    name = "check-deprule",
    about = "Lint dependency constraints in Cargo workspaces"
)]
struct Cli {
    /// Path to Cargo.toml
    #[arg(long)]
    manifest_path: Option<String>,

    /// Path to dependency_rules.toml
    #[arg(long)]
    rules_path: Option<PathBuf>,

    /// Exclude dev-dependencies from the graph
    #[arg(long)]
    no_dev_dependencies: bool,

    /// Tree character set
    #[arg(long, value_enum, default_value_t = Charset::Utf8)]
    charset: Charset,

    /// Tree prefix style
    #[arg(long, value_enum, default_value_t = Prefix::Indent)]
    prefix: Prefix,

    /// Log level (overridden by RUST_LOG env var)
    #[arg(long, default_value = "warn")]
    log_level: tracing::Level,
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(cli.log_level.as_str())),
        )
        .init();

    info!(
        manifest_path = ?cli.manifest_path,
        no_dev_dependencies = cli.no_dev_dependencies,
        "starting check-deprule"
    );

    let config = HandlerConfig {
        graph_build_configs: DependencyGraphBuildConfigs::new(cli.no_dev_dependencies),
        metadata_configs: CollectMetadataConfig {
            manifest_path: cli.manifest_path,
            ..CollectMetadataConfig::default()
        },
        tree_config: TreePrintConfig {
            charset: cli.charset,
            prefix: cli.prefix,
        },
        rules_path: cli.rules_path,
    };

    let result = handler(config)?;

    Ok(result.to_return_code())
}
