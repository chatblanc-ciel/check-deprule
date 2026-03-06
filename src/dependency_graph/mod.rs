use anyhow::{Error, anyhow};
use cargo_metadata::{DependencyKind, Metadata, Package, PackageId};
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::Dfs;
use std::collections::HashMap;

pub(crate) mod formatter;
pub mod tree;

#[derive(Debug, Clone)]
pub struct Graph {
    pub graph: StableGraph<Package, DependencyKind>,
    pub nodes: HashMap<PackageId, NodeIndex>,
    pub root: Option<PackageId>,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyGraphBuildConfigs {
    no_dev_dependencies: bool,
}
impl DependencyGraphBuildConfigs {
    pub fn new(no_dev_dependencies: bool) -> Self {
        Self {
            no_dev_dependencies,
        }
    }
}

#[tracing::instrument(skip(metadata), fields(packages = metadata.packages.len()))]
pub fn build_dependency_graph(
    metadata: Metadata,
    config: DependencyGraphBuildConfigs,
) -> Result<Graph, Error> {
    let resolve = metadata
        .resolve
        .ok_or_else(|| anyhow!("cargo metadata did not return dependency resolve information"))?;

    let mut graph = Graph {
        graph: StableGraph::new(),
        nodes: HashMap::new(),
        root: resolve.root,
    };

    for package in metadata.packages {
        let id = package.id.clone();
        let index = graph.graph.add_node(package);
        graph.nodes.insert(id, index);
    }

    for node in resolve.nodes {
        if node.deps.len() != node.dependencies.len() {
            return Err(anyhow!("cargo tree requires cargo 1.41 or newer"));
        }

        let from = graph.nodes[&node.id];
        for dep in node.deps {
            if dep.dep_kinds.is_empty() {
                return Err(anyhow!("cargo tree requires cargo 1.41 or newer"));
            }

            // https://github.com/rust-lang/cargo/issues/7752
            let mut kinds = vec![];
            for kind in dep.dep_kinds {
                if !kinds.contains(&kind.kind) {
                    kinds.push(kind.kind);
                }
            }

            let to = graph.nodes[&dep.pkg];
            for kind in kinds {
                if config.no_dev_dependencies && kind == DependencyKind::Development {
                    continue;
                }

                graph.graph.add_edge(from, to, kind);
            }
        }
    }

    // prune nodes not reachable from the root package (directionally)
    if let Some(root) = &graph.root {
        let mut dfs = Dfs::new(&graph.graph, graph.nodes[root]);
        while dfs.next(&graph.graph).is_some() {}

        let g = &mut graph.graph;
        graph.nodes.retain(|_, idx| {
            if !dfs.discovered.contains(idx.index()) {
                g.remove_node(*idx);
                false
            } else {
                true
            }
        });
    }

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{CollectMetadataConfig, collect_metadata};
    use anyhow::Result;

    fn clean_arch_metadata() -> Result<Metadata> {
        collect_metadata(CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        })
    }

    #[test]
    fn test_build_dependency_graph_creates_nodes() -> Result<()> {
        let metadata = clean_arch_metadata()?;
        let workspace_count = metadata.workspace_members.len();
        let graph = build_dependency_graph(metadata, DependencyGraphBuildConfigs::default())?;

        assert!(graph.nodes.len() >= workspace_count);
        Ok(())
    }

    #[test]
    fn test_build_dependency_graph_workspace_members_present() -> Result<()> {
        let metadata = clean_arch_metadata()?;
        let member_ids: Vec<_> = metadata.workspace_members.clone();
        let graph = build_dependency_graph(metadata, DependencyGraphBuildConfigs::default())?;

        for id in &member_ids {
            assert!(
                graph.nodes.contains_key(id),
                "missing workspace member: {id}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_build_dependency_graph_no_dev_dependencies() -> Result<()> {
        let metadata = clean_arch_metadata()?;
        let config_with_dev = DependencyGraphBuildConfigs::new(false);
        let graph_with_dev = build_dependency_graph(metadata.clone(), config_with_dev)?;

        let config_no_dev = DependencyGraphBuildConfigs::new(true);
        let graph_no_dev = build_dependency_graph(metadata, config_no_dev)?;

        assert!(
            graph_no_dev.graph.edge_count() <= graph_with_dev.graph.edge_count(),
            "no_dev_dependencies should result in equal or fewer edges"
        );
        Ok(())
    }

    #[test]
    fn test_dependency_graph_build_configs_default() {
        let config = DependencyGraphBuildConfigs::default();
        assert!(!config.no_dev_dependencies);
    }

    #[test]
    fn test_dependency_graph_build_configs_new() {
        let config = DependencyGraphBuildConfigs::new(true);
        assert!(config.no_dev_dependencies);

        let config = DependencyGraphBuildConfigs::new(false);
        assert!(!config.no_dev_dependencies);
    }
}
