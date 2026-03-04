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

// TODO: dead code回避を精査すること

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Prefix {
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

#[allow(dead_code)]
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

pub fn print(
    graph: &Graph,
    metadata: &Metadata,
    rules: DependencyRules,
) -> Result<ReturnStatus, Error> {
    let format = Pattern::new("{p}")?;
    let direction = EdgeDirection::Outgoing;
    let symbols = &UTF8_SYMBOLS;
    let prefix = Prefix::Indent;
    let mut return_status = ReturnStatus::NoViolation;

    for member_id in &metadata.workspace_members {
        let idx = graph.nodes.get(member_id).ok_or_else(|| {
            anyhow!("workspace member `{member_id}` not found in dependency graph")
        })?;
        let root = &graph.graph[*idx];

        let result = print_tree(
            graph, root, &format, direction, symbols, prefix, true, &rules,
        );

        if let ReturnStatus::Violation = result {
            return_status = ReturnStatus::Violation
        }
    }

    Ok(return_status)
}

#[allow(clippy::too_many_arguments)]
fn print_tree<'a>(
    graph: &'a Graph,
    root: &'a Package,
    format: &Pattern,
    direction: EdgeDirection,
    symbols: &Symbols,
    prefix: Prefix,
    all: bool,
    rules: &DependencyRules,
) -> ReturnStatus {
    let mut visited_deps = HashSet::new();
    let mut levels_continue = vec![];

    print_package(
        graph,
        None,
        root,
        format,
        direction,
        symbols,
        prefix,
        all,
        &mut visited_deps,
        &mut levels_continue,
        rules,
        ReturnStatus::NoViolation,
    )
}

// TODO: lint回避の精査
#[allow(clippy::too_many_arguments)]
fn print_package<'a>(
    graph: &'a Graph,
    parent_package: Option<&'a Package>,
    package: &'a Package,
    format: &Pattern,
    direction: EdgeDirection,
    symbols: &Symbols,
    prefix: Prefix,
    all: bool,
    visited_deps: &mut HashSet<&'a PackageId>,
    levels_continue: &mut Vec<bool>,
    rules: &DependencyRules,
    parent_return_status: ReturnStatus,
) -> ReturnStatus {
    let new = all || visited_deps.insert(&package.id);

    match prefix {
        Prefix::Depth => print!("{}", levels_continue.len()),
        Prefix::Indent => {
            if let Some((last_continues, rest)) = levels_continue.split_last() {
                for continues in rest {
                    let c = if *continues { symbols.down } else { " " };
                    print!("{c}   ");
                }

                let c = if *last_continues {
                    symbols.tee
                } else {
                    symbols.ell
                };
                print!("{0}{1}{1} ", c, symbols.right);
            }
        }
        Prefix::None => {}
    }

    let star = if new { "" } else { " (*)" };
    let mut is_violation = {
        rules.rules.iter().any(|rule| {
            // println!("\npackage: {}\nid: {}", rule.package, package.id);
            // println!("rule.package: {}", rule.package);
            // println!("parent_package: {:?}", parent_package);
            // println!("forbidden_dependencies: {:?}", rule.forbidden_dependencies);
            if let Some(parent_package) = parent_package {
                // println!("parent match: {}", rule.package == PackageId { repr: parent_package.name.clone() });
                // println!("forbidden_dependencies match: {}", rule.forbidden_dependencies.contains(&PackageId { repr: package.name.clone() }));
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
        false => println!("{}{}", format.display(package), star),
    };

    if !new {
        return match (is_violation, parent_return_status) {
            (true, _) => ReturnStatus::Violation,
            (false, ReturnStatus::Violation) => ReturnStatus::Violation,
            _ => ReturnStatus::NoViolation,
        };
    }

    for kind in &[
        DependencyKind::Normal,
        DependencyKind::Build,
        DependencyKind::Development,
    ] {
        let current_return_status = match (is_violation, parent_return_status.clone()) {
            (true, _) => ReturnStatus::Violation,
            (false, ReturnStatus::Violation) => ReturnStatus::Violation,
            _ => ReturnStatus::NoViolation,
        };

        let result = print_dependencies(
            graph,
            package,
            format,
            direction,
            symbols,
            prefix,
            all,
            visited_deps,
            levels_continue,
            *kind,
            rules,
            current_return_status,
        );

        if let ReturnStatus::Violation = result {
            is_violation = true;
        }
    }

    match (is_violation, parent_return_status) {
        (true, _) => ReturnStatus::Violation,
        (false, ReturnStatus::Violation) => ReturnStatus::Violation,
        _ => ReturnStatus::NoViolation,
    }
}

// TODO: lint回避の精査
#[allow(clippy::too_many_arguments)]
fn print_dependencies<'a>(
    graph: &'a Graph,
    package: &'a Package,
    format: &Pattern,
    direction: EdgeDirection,
    symbols: &Symbols,
    prefix: Prefix,
    all: bool,
    visited_deps: &mut HashSet<&'a PackageId>,
    levels_continue: &mut Vec<bool>,
    kind: DependencyKind,
    rules: &DependencyRules,
    parent_return_status: ReturnStatus,
) -> ReturnStatus {
    let idx = graph.nodes[&package.id];
    let mut deps = vec![];
    for edge in graph.graph.edges_directed(idx, direction) {
        if *edge.weight() != kind {
            continue;
        }

        let dep = match direction {
            EdgeDirection::Incoming => &graph.graph[edge.source()],
            EdgeDirection::Outgoing => &graph.graph[edge.target()],
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

    if let Prefix::Indent = prefix {
        if let Some(name) = name {
            for continues in &**levels_continue {
                let c = if *continues { symbols.down } else { " " };
                print!("{c}   ");
            }

            println!("{name}");
        }
    }

    let mut is_violation = false;
    let mut it = deps.iter().peekable();
    while let Some(dependency) = it.next() {
        levels_continue.push(it.peek().is_some());
        let current_return_status = if is_violation {
            ReturnStatus::Violation
        } else {
            parent_return_status.clone()
        };

        let result = print_package(
            graph,
            Some(package),
            dependency,
            format,
            direction,
            symbols,
            prefix,
            all,
            visited_deps,
            levels_continue,
            rules,
            current_return_status,
        );

        levels_continue.pop();
        if let ReturnStatus::Violation = result {
            is_violation = true;
        }
    }

    match (is_violation, parent_return_status) {
        (true, _) => ReturnStatus::Violation,
        (false, ReturnStatus::Violation) => ReturnStatus::Violation,
        _ => ReturnStatus::NoViolation,
    }
}
