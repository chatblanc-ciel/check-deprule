use super::Graph;
use super::formatter::Pattern;
use anyhow::{Context, Error, anyhow};
use cargo::core::Workspace;
use cargo::util::context::GlobalContext;
use cargo_metadata::{DependencyKind, Package, PackageId};
use petgraph::EdgeDirection;
use petgraph::visit::EdgeRef;
use semver::Version;
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

pub fn print(graph: &Graph) -> Result<(), Error> {
    let glcx = GlobalContext::default()?;
    let manifest_path = std::path::Path::new("tests/demo_crates/clean-arch/Cargo.toml");
    let ws = Workspace::new(std::path::absolute(manifest_path)?.as_path(), &glcx)?;

    let format = Pattern::new("{p}")?;
    let direction = EdgeDirection::Outgoing;
    let symbols = &UTF8_SYMBOLS;
    let prefix = Prefix::Indent;

    for package in ws.members() {
        let root = find_package(package.name().as_str(), graph)?;
        let root = &graph.graph[graph.nodes[root]];

        print_tree(graph, root, &format, direction, symbols, prefix, true);
    }

    Ok(())
}

fn find_package<'a>(package: &str, graph: &'a Graph) -> Result<&'a PackageId, Error> {
    let mut it = package.split(':');
    let name = it.next().unwrap();
    let version = it
        .next()
        .map(Version::parse)
        .transpose()
        .context("error parsing package version")?;

    let mut candidates = vec![];
    for idx in graph.graph.node_indices() {
        let package = &graph.graph[idx];
        if package.name != name {
            continue;
        }

        if let Some(version) = &version {
            if package.version != *version {
                continue;
            }
        }

        candidates.push(package);
    }

    if candidates.is_empty() {
        Err(anyhow!("no crates found for package `{}`", package))
    } else if candidates.len() > 1 {
        let specs = candidates
            .iter()
            .map(|p| format!("{}:{}", p.name, p.version))
            .collect::<Vec<_>>()
            .join(", ");
        Err(anyhow!(
            "multiple crates found for package `{}`: {}",
            package,
            specs,
        ))
    } else {
        Ok(&candidates[0].id)
    }
}

fn print_tree<'a>(
    graph: &'a Graph,
    root: &'a Package,
    format: &Pattern,
    direction: EdgeDirection,
    symbols: &Symbols,
    prefix: Prefix,
    all: bool,
) {
    let mut visited_deps = HashSet::new();
    let mut levels_continue = vec![];

    print_package(
        graph,
        root,
        format,
        direction,
        symbols,
        prefix,
        all,
        &mut visited_deps,
        &mut levels_continue,
    );
}

// TODO: lint回避の精査
#[allow(clippy::too_many_arguments)]
fn print_package<'a>(
    graph: &'a Graph,
    package: &'a Package,
    format: &Pattern,
    direction: EdgeDirection,
    symbols: &Symbols,
    prefix: Prefix,
    all: bool,
    visited_deps: &mut HashSet<&'a PackageId>,
    levels_continue: &mut Vec<bool>,
) {
    let new = all || visited_deps.insert(&package.id);

    match prefix {
        Prefix::Depth => print!("{}", levels_continue.len()),
        Prefix::Indent => {
            if let Some((last_continues, rest)) = levels_continue.split_last() {
                for continues in rest {
                    let c = if *continues { symbols.down } else { " " };
                    print!("{}   ", c);
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
    println!("{}{}", format.display(package), star);

    if !new {
        return;
    }

    for kind in &[
        DependencyKind::Normal,
        DependencyKind::Build,
        DependencyKind::Development,
    ] {
        print_dependencies(
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
        );
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
) {
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
        return;
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
                print!("{}   ", c);
            }

            println!("{}", name);
        }
    }

    let mut it = deps.iter().peekable();
    while let Some(dependency) = it.next() {
        levels_continue.push(it.peek().is_some());
        print_package(
            graph,
            dependency,
            format,
            direction,
            symbols,
            prefix,
            all,
            visited_deps,
            levels_continue,
        );
        levels_continue.pop();
    }
}
