use crate::ReturnStatus;
use crate::dependency_graph::formatter::Chunk;
use crate::dependency_rule::DependencyRules;

use super::Graph;
use super::formatter::Pattern;
use anyhow::{Error, anyhow};
use cargo_metadata::{DependencyKind, Metadata, Package, PackageId};
use petgraph::EdgeDirection;
use petgraph::visit::EdgeRef;
use std::collections::HashSet;
use std::io::Write;

#[derive(Clone, Copy)]
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
    rules: &'a DependencyRules,
    visited_deps: HashSet<&'a PackageId>,
    levels_continue: Vec<bool>,
}

impl<'a, W: Write> TreePrinter<'a, W> {
    fn new(
        writer: W,
        graph: &'a Graph,
        rules: &'a DependencyRules,
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
            rules,
            visited_deps: HashSet::new(),
            levels_continue: vec![],
        })
    }

    fn print_root(&mut self, root: &'a Package) -> Result<ReturnStatus, Error> {
        self.visited_deps.clear();
        self.levels_continue.clear();
        self.print_package(None, root, ReturnStatus::NoViolation)
    }

    fn print_package(
        &mut self,
        parent_package: Option<&'a Package>,
        package: &'a Package,
        parent_return_status: ReturnStatus,
    ) -> Result<ReturnStatus, Error> {
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
        let mut is_violation = {
            self.rules.rules.iter().any(|rule| {
                if let Some(parent_package) = parent_package {
                    rule.package
                        == PackageId {
                            repr: parent_package.name.clone(),
                        }
                        && rule.forbidden_dependencies.contains(&PackageId {
                            repr: package.name.clone(),
                        })
                } else {
                    false
                }
            })
        };
        match is_violation {
            true => {
                let f = Pattern(vec![Chunk::ViolationPackage]);
                writeln!(self.writer, "{}{}", f.display(package), star)?;
            }
            false => writeln!(self.writer, "{}{}", self.format.display(package), star)?,
        };

        if !new {
            return Ok(parent_return_status.merge(is_violation));
        }

        for kind in &[
            DependencyKind::Normal,
            DependencyKind::Build,
            DependencyKind::Development,
        ] {
            let current_return_status = parent_return_status.clone().merge(is_violation);

            let result = self.print_dependencies(package, *kind, current_return_status)?;

            if let ReturnStatus::Violation = result {
                is_violation = true;
            }
        }

        Ok(parent_return_status.merge(is_violation))
    }

    fn print_dependencies(
        &mut self,
        package: &'a Package,
        kind: DependencyKind,
        parent_return_status: ReturnStatus,
    ) -> Result<ReturnStatus, Error> {
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
            return Ok(parent_return_status);
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

        let mut is_violation = false;
        let mut it = deps.iter().peekable();
        while let Some(dependency) = it.next() {
            self.levels_continue.push(it.peek().is_some());
            let current_return_status = if is_violation {
                ReturnStatus::Violation
            } else {
                parent_return_status.clone()
            };

            let result = self.print_package(Some(package), dependency, current_return_status)?;

            self.levels_continue.pop();
            if let ReturnStatus::Violation = result {
                is_violation = true;
            }
        }

        Ok(parent_return_status.merge(is_violation))
    }
}

pub fn print(
    writer: &mut impl Write,
    graph: &Graph,
    metadata: &Metadata,
    rules: DependencyRules,
    config: TreePrintConfig,
) -> Result<ReturnStatus, Error> {
    let mut printer = TreePrinter::new(writer, graph, &rules, config)?;
    let mut return_status = ReturnStatus::NoViolation;

    for member_id in &metadata.workspace_members {
        let idx = graph.nodes.get(member_id).ok_or_else(|| {
            anyhow!("workspace member `{member_id}` not found in dependency graph")
        })?;
        let root = &graph.graph[*idx];

        let result = printer.print_root(root)?;

        if let ReturnStatus::Violation = result {
            return_status = ReturnStatus::Violation
        }
    }

    Ok(return_status)
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let mut buf = Vec::new();
        let result = print(
            &mut buf,
            &graph,
            &metadata,
            rules,
            TreePrintConfig::default(),
        )?;

        assert!(!buf.is_empty());
        assert!(matches!(result, ReturnStatus::NoViolation));
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

        let mut buf = Vec::new();
        let result = print(
            &mut buf,
            &graph,
            &metadata,
            rules,
            TreePrintConfig::default(),
        )?;

        assert!(!buf.is_empty());
        assert!(matches!(result, ReturnStatus::Violation));
        Ok(())
    }
}
