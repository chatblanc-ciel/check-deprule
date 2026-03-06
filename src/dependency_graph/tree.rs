use crate::dependency_graph::formatter::Chunk;
use crate::dependency_graph::violation::ViolationReport;

use super::Graph;
use super::formatter::Pattern;
use anyhow::{Error, anyhow};
use cargo_metadata::{DependencyKind, Metadata, Package, PackageId};
use petgraph::EdgeDirection;
use petgraph::visit::EdgeRef;
use std::collections::HashSet;
use std::io::Write;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Prefix {
    None,
    Indent,
    Depth,
}

struct Symbols {
    down: &'static str,
    tee: &'static str,
    ell: &'static str,
    right: &'static str,
}

static UTF8_SYMBOLS: Symbols = Symbols {
    down: "│",
    tee: "├",
    ell: "└",
    right: "─",
};

static ASCII_SYMBOLS: Symbols = Symbols {
    down: "|",
    tee: "|",
    ell: "`",
    right: "-",
};

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Charset {
    Utf8,
    Ascii,
}

impl Charset {
    fn symbols(&self) -> &'static Symbols {
        match self {
            Charset::Utf8 => &UTF8_SYMBOLS,
            Charset::Ascii => &ASCII_SYMBOLS,
        }
    }
}

impl std::str::FromStr for Charset {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Charset, &'static str> {
        match s {
            "utf8" => Ok(Charset::Utf8),
            "ascii" => Ok(Charset::Ascii),
            _ => Err("invalid charset"),
        }
    }
}

pub struct TreePrintConfig {
    pub charset: Charset,
    pub prefix: Prefix,
}

impl Default for TreePrintConfig {
    fn default() -> Self {
        Self {
            charset: Charset::Utf8,
            prefix: Prefix::Indent,
        }
    }
}

struct TreePrinter<'a, W: Write> {
    writer: W,
    graph: &'a Graph,
    format: Pattern,
    direction: EdgeDirection,
    symbols: &'static Symbols,
    prefix: Prefix,
    all: bool,
    report: &'a ViolationReport,
    visited_deps: HashSet<&'a PackageId>,
    levels_continue: Vec<bool>,
}

impl<'a, W: Write> TreePrinter<'a, W> {
    fn new(
        writer: W,
        graph: &'a Graph,
        report: &'a ViolationReport,
        config: TreePrintConfig,
    ) -> Result<Self, Error> {
        Ok(Self {
            writer,
            graph,
            format: Pattern::new("{p}")?,
            direction: EdgeDirection::Outgoing,
            symbols: config.charset.symbols(),
            prefix: config.prefix,
            all: true,
            report,
            visited_deps: HashSet::new(),
            levels_continue: vec![],
        })
    }

    fn print_root(&mut self, root: &'a Package) -> Result<(), Error> {
        self.visited_deps.clear();
        self.levels_continue.clear();
        self.print_package(None, root)
    }

    fn print_package(
        &mut self,
        parent_package: Option<&'a Package>,
        package: &'a Package,
    ) -> Result<(), Error> {
        let new = self.all || self.visited_deps.insert(&package.id);

        match self.prefix {
            Prefix::Depth => write!(self.writer, "{}", self.levels_continue.len())?,
            Prefix::Indent => {
                if let Some((last_continues, rest)) = self.levels_continue.split_last() {
                    for continues in rest {
                        let c = if *continues { self.symbols.down } else { " " };
                        write!(self.writer, "{c}   ")?;
                    }

                    let c = if *last_continues {
                        self.symbols.tee
                    } else {
                        self.symbols.ell
                    };
                    write!(self.writer, "{0}{1}{1} ", c, self.symbols.right)?;
                }
            }
            Prefix::None => {}
        }

        let star = if new { "" } else { " (*)" };
        let is_violation = if let Some(parent) = parent_package {
            self.report.is_violation(&parent.name, &package.name)
        } else {
            false
        };
        match is_violation {
            true => {
                let f = Pattern(vec![Chunk::ViolationPackage]);
                writeln!(self.writer, "{}{}", f.display(package), star)?;
            }
            false => writeln!(self.writer, "{}{}", self.format.display(package), star)?,
        };

        if !new {
            return Ok(());
        }

        for kind in &[
            DependencyKind::Normal,
            DependencyKind::Build,
            DependencyKind::Development,
        ] {
            self.print_dependencies(package, *kind)?;
        }

        Ok(())
    }

    fn print_dependencies(
        &mut self,
        package: &'a Package,
        kind: DependencyKind,
    ) -> Result<(), Error> {
        let idx = self.graph.nodes[&package.id];
        let mut deps = vec![];
        for edge in self.graph.graph.edges_directed(idx, self.direction) {
            let weight: &DependencyKind = edge.weight();
            if *weight != kind {
                continue;
            }

            let dep = match self.direction {
                EdgeDirection::Incoming => &self.graph.graph[edge.source()],
                EdgeDirection::Outgoing => &self.graph.graph[edge.target()],
            };
            deps.push(dep);
        }

        if deps.is_empty() {
            return Ok(());
        }

        // ensure a consistent output ordering
        deps.sort_by_key(|p| &p.id);

        let name = match kind {
            DependencyKind::Normal => None,
            DependencyKind::Build => Some("[build-dependencies]"),
            DependencyKind::Development => Some("[dev-dependencies]"),
            _ => unreachable!(),
        };

        if let Prefix::Indent = self.prefix
            && let Some(name) = name
        {
            for continues in &*self.levels_continue {
                let c = if *continues { self.symbols.down } else { " " };
                write!(self.writer, "{c}   ")?;
            }

            writeln!(self.writer, "{name}")?;
        }

        let mut it = deps.iter().peekable();
        while let Some(dependency) = it.next() {
            self.levels_continue.push(it.peek().is_some());
            self.print_package(Some(package), dependency)?;
            self.levels_continue.pop();
        }

        Ok(())
    }
}

pub fn print(
    writer: &mut impl Write,
    graph: &Graph,
    metadata: &Metadata,
    report: &ViolationReport,
    config: TreePrintConfig,
) -> Result<(), Error> {
    let mut printer = TreePrinter::new(writer, graph, report, config)?;

    for member_id in &metadata.workspace_members {
        let idx = graph.nodes.get(member_id).ok_or_else(|| {
            anyhow!("workspace member `{member_id}` not found in dependency graph")
        })?;
        let root = &graph.graph[*idx];

        printer.print_root(root)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_graph::violation::check_violations;
    use crate::dependency_graph::{DependencyGraphBuildConfigs, build_dependency_graph};
    use crate::dependency_rule::DependencyRules;
    use crate::metadata::{CollectMetadataConfig, collect_metadata};
    use anyhow::Result;

    #[test]
    fn test_print_no_violation_writes_output() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph =
            build_dependency_graph(metadata.clone(), DependencyGraphBuildConfigs::default())?;
        let rules =
            DependencyRules::from_file("tests/demo_crates/clean-arch/dependency_rules.toml")?;
        let report = check_violations(&graph, &rules);

        let mut buf = Vec::new();
        print(
            &mut buf,
            &graph,
            &metadata,
            &report,
            TreePrintConfig::default(),
        )?;

        assert!(!buf.is_empty());
        assert!(!report.has_violations());
        Ok(())
    }

    #[test]
    fn test_print_with_violation_writes_output() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/tangled-clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph =
            build_dependency_graph(metadata.clone(), DependencyGraphBuildConfigs::default())?;
        let rules = DependencyRules::from_file(
            "tests/demo_crates/tangled-clean-arch/dependency_rules.toml",
        )?;
        let report = check_violations(&graph, &rules);

        let mut buf = Vec::new();
        print(
            &mut buf,
            &graph,
            &metadata,
            &report,
            TreePrintConfig::default(),
        )?;

        assert!(!buf.is_empty());
        assert!(report.has_violations());
        Ok(())
    }

    #[test]
    fn test_print_output_contains_workspace_members() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph =
            build_dependency_graph(metadata.clone(), DependencyGraphBuildConfigs::default())?;
        let rules =
            DependencyRules::from_file("tests/demo_crates/clean-arch/dependency_rules.toml")?;
        let report = check_violations(&graph, &rules);

        let mut buf = Vec::new();
        print(
            &mut buf,
            &graph,
            &metadata,
            &report,
            TreePrintConfig::default(),
        )?;

        let output = String::from_utf8(buf)?;
        assert!(output.contains("ca-core"));
        assert!(output.contains("ca-interactor"));
        Ok(())
    }

    #[test]
    fn test_print_ascii_charset() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph =
            build_dependency_graph(metadata.clone(), DependencyGraphBuildConfigs::default())?;
        let rules =
            DependencyRules::from_file("tests/demo_crates/clean-arch/dependency_rules.toml")?;
        let report = check_violations(&graph, &rules);

        let mut buf = Vec::new();
        let tree_config = TreePrintConfig {
            charset: Charset::Ascii,
            prefix: Prefix::Indent,
        };
        print(&mut buf, &graph, &metadata, &report, tree_config)?;

        let output = String::from_utf8(buf)?;
        assert!(output.contains("|--"), "ASCII tree should contain |--");
        Ok(())
    }

    #[test]
    fn test_print_depth_prefix() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph =
            build_dependency_graph(metadata.clone(), DependencyGraphBuildConfigs::default())?;
        let rules =
            DependencyRules::from_file("tests/demo_crates/clean-arch/dependency_rules.toml")?;
        let report = check_violations(&graph, &rules);

        let mut buf = Vec::new();
        let tree_config = TreePrintConfig {
            charset: Charset::Utf8,
            prefix: Prefix::Depth,
        };
        print(&mut buf, &graph, &metadata, &report, tree_config)?;

        let output = String::from_utf8(buf)?;
        assert!(
            output.contains("0"),
            "Depth prefix should start with depth 0"
        );
        Ok(())
    }

    #[test]
    fn test_print_no_prefix() -> Result<()> {
        let config = CollectMetadataConfig {
            manifest_path: Some("tests/demo_crates/clean-arch/Cargo.toml".to_string()),
            ..CollectMetadataConfig::default()
        };
        let metadata = collect_metadata(config)?;
        let graph =
            build_dependency_graph(metadata.clone(), DependencyGraphBuildConfigs::default())?;
        let rules =
            DependencyRules::from_file("tests/demo_crates/clean-arch/dependency_rules.toml")?;
        let report = check_violations(&graph, &rules);

        let mut buf = Vec::new();
        let tree_config = TreePrintConfig {
            charset: Charset::Utf8,
            prefix: Prefix::None,
        };
        print(&mut buf, &graph, &metadata, &report, tree_config)?;

        let output = String::from_utf8(buf)?;
        assert!(
            !output.contains("├"),
            "None prefix should not contain tree symbols"
        );
        assert!(
            !output.contains("└"),
            "None prefix should not contain tree symbols"
        );
        Ok(())
    }

    #[test]
    fn test_charset_from_str() {
        assert!(matches!("utf8".parse::<Charset>(), Ok(Charset::Utf8)));
        assert!(matches!("ascii".parse::<Charset>(), Ok(Charset::Ascii)));
        assert!("invalid".parse::<Charset>().is_err());
    }

    #[test]
    fn test_tree_print_config_default() {
        let config = TreePrintConfig::default();
        assert!(matches!(config.charset, Charset::Utf8));
        assert!(matches!(config.prefix, Prefix::Indent));
    }
}
