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

struct TreePrinter<'a> {
    graph: &'a Graph,
    format: Pattern,
    direction: EdgeDirection,
    symbols: &'static Symbols,
    all: bool,
    rules: &'a DependencyRules,
    visited_deps: HashSet<&'a PackageId>,
    levels_continue: Vec<bool>,
}

impl<'a> TreePrinter<'a> {
    fn new(graph: &'a Graph, rules: &'a DependencyRules) -> Result<Self, Error> {
        Ok(Self {
            graph,
            format: Pattern::new("{p}")?,
            direction: EdgeDirection::Outgoing,
            symbols: &UTF8_SYMBOLS,
            all: true,
            rules,
            visited_deps: HashSet::new(),
            levels_continue: vec![],
        })
    }

    fn print_root(&mut self, root: &'a Package) -> ReturnStatus {
        self.visited_deps.clear();
        self.levels_continue.clear();
        self.print_package(None, root, ReturnStatus::NoViolation)
    }

    fn print_package(
        &mut self,
        parent_package: Option<&'a Package>,
        package: &'a Package,
        parent_return_status: ReturnStatus,
    ) -> ReturnStatus {
        let new = self.all || self.visited_deps.insert(&package.id);

        if let Some((last_continues, rest)) = self.levels_continue.split_last() {
            for continues in rest {
                let c = if *continues { self.symbols.down } else { " " };
                print!("{c}   ");
            }

            let c = if *last_continues {
                self.symbols.tee
            } else {
                self.symbols.ell
            };
            print!("{0}{1}{1} ", c, self.symbols.right);
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
                println!("{}{}", f.display(package), star);
            }
            false => println!("{}{}", self.format.display(package), star),
        };

        if !new {
            return parent_return_status.merge(is_violation);
        }

        for kind in &[
            DependencyKind::Normal,
            DependencyKind::Build,
            DependencyKind::Development,
        ] {
            let current_return_status = parent_return_status.clone().merge(is_violation);

            let result = self.print_dependencies(package, *kind, current_return_status);

            if let ReturnStatus::Violation = result {
                is_violation = true;
            }
        }

        parent_return_status.merge(is_violation)
    }

    fn print_dependencies(
        &mut self,
        package: &'a Package,
        kind: DependencyKind,
        parent_return_status: ReturnStatus,
    ) -> ReturnStatus {
        let idx = self.graph.nodes[&package.id];
        let mut deps = vec![];
        for edge in self.graph.graph.edges_directed(idx, self.direction) {
            if *edge.weight() != kind {
                continue;
            }

            let dep = match self.direction {
                EdgeDirection::Incoming => &self.graph.graph[edge.source()],
                EdgeDirection::Outgoing => &self.graph.graph[edge.target()],
            };
            deps.push(dep);
        }

        if deps.is_empty() {
            return parent_return_status;
        }

        // ensure a consistent output ordering
        deps.sort_by_key(|p| &p.id);

        let name = match kind {
            DependencyKind::Normal => None,
            DependencyKind::Build => Some("[build-dependencies]"),
            DependencyKind::Development => Some("[dev-dependencies]"),
            _ => unreachable!(),
        };

        if let Some(name) = name {
            for continues in &*self.levels_continue {
                let c = if *continues { self.symbols.down } else { " " };
                print!("{c}   ");
            }

            println!("{name}");
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

            let result = self.print_package(Some(package), dependency, current_return_status);

            self.levels_continue.pop();
            if let ReturnStatus::Violation = result {
                is_violation = true;
            }
        }

        parent_return_status.merge(is_violation)
    }
}

pub fn print(
    graph: &Graph,
    metadata: &Metadata,
    rules: DependencyRules,
) -> Result<ReturnStatus, Error> {
    let mut printer = TreePrinter::new(graph, &rules)?;
    let mut return_status = ReturnStatus::NoViolation;

    for member_id in &metadata.workspace_members {
        let idx = graph.nodes.get(member_id).ok_or_else(|| {
            anyhow!("workspace member `{member_id}` not found in dependency graph")
        })?;
        let root = &graph.graph[*idx];

        let result = printer.print_root(root);

        if let ReturnStatus::Violation = result {
            return_status = ReturnStatus::Violation
        }
    }

    Ok(return_status)
}
