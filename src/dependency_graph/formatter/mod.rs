use anyhow::{Error, anyhow};
use cargo_metadata::Package;
use parse::{Parser, RawChunk};
use std::fmt;

mod parse;

enum Chunk {
    Raw(String),
    Package,
    License,
    Repository,
}

pub struct Pattern(Vec<Chunk>);

impl Pattern {
    pub fn new(format: &str) -> Result<Pattern, Error> {
        let mut chunks = vec![];

        for raw in Parser::new(format) {
            let chunk = match raw {
                RawChunk::Text(text) => Chunk::Raw(text.to_owned()),
                RawChunk::Argument("p") => Chunk::Package,
                RawChunk::Argument("l") => Chunk::License,
                RawChunk::Argument("r") => Chunk::Repository,
                RawChunk::Argument(ref a) => {
                    return Err(anyhow!("unsupported pattern `{}`", a));
                }
                RawChunk::Error(err) => return Err(anyhow!("{}", err)),
            };
            chunks.push(chunk);
        }

        Ok(Pattern(chunks))
    }

    pub fn display<'a>(&'a self, package: &'a Package) -> Display<'a> {
        Display {
            pattern: self,
            package,
        }
    }
}

pub struct Display<'a> {
    pattern: &'a Pattern,
    package: &'a Package,
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in &self.pattern.0 {
            match *chunk {
                Chunk::Raw(ref s) => fmt.write_str(s)?,
                Chunk::Package => {
                    write!(fmt, "{} v{}", self.package.name, self.package.version)?;

                    match &self.package.source {
                        Some(source) if !source.is_crates_io() => write!(fmt, " ({})", source)?,
                        // https://github.com/rust-lang/cargo/issues/7483
                        None => write!(
                            fmt,
                            " ({})",
                            self.package.manifest_path.parent().unwrap().as_str()
                        )?,
                        _ => {}
                    }
                }
                Chunk::License => {
                    if let Some(ref license) = self.package.license {
                        write!(fmt, "{}", license)?
                    }
                }
                Chunk::Repository => {
                    if let Some(ref repository) = self.package.repository {
                        write!(fmt, "{}", repository)?
                    }
                }
            }
        }

        Ok(())
    }
}
