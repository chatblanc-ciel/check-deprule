use anyhow::{Context, Error, anyhow};
use cargo_metadata::Metadata;
use std::env;
use std::ffi::OsString;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct CollectMetadataConfig {
    pub quiet: bool,
    pub features: Option<String>,
    pub all_features: bool,
    pub no_default_features: bool,
    pub all_targets: bool,
    pub target: Option<String>,
    pub manifest_path: Option<String>,
    pub verbose: u32,
    pub color: Option<String>,
    pub frozen: bool,
    pub locked: bool,
    pub offline: bool,
    pub unstable_flags: Vec<String>,
}
impl Default for CollectMetadataConfig {
    fn default() -> Self {
        Self {
            quiet: false,
            features: None,
            all_features: true,
            no_default_features: false,
            all_targets: true,
            target: None,
            manifest_path: None,
            verbose: 0,
            color: None,
            frozen: false,
            locked: false,
            offline: false,
            unstable_flags: Vec::new(),
        }
    }
}

pub fn collect_metadata(config: CollectMetadataConfig) -> Result<Metadata, Error> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));

    let mut command = Command::new(cargo);
    command.arg("metadata").arg("--format-version").arg("1");

    if config.quiet {
        command.arg("-q");
    }

    if let Some(features) = &config.features {
        command.arg("--features").arg(features);
    }
    if config.all_features {
        command.arg("--all-features");
    }
    if config.no_default_features {
        command.arg("--no-default-features");
    }

    if !config.all_targets {
        command.arg("--filter-platform");
        match &config.target {
            Some(target) => {
                command.arg(target);
            }
            None => {
                let target = default_target()?;
                command.arg(target);
            }
        }
    }

    if let Some(path) = &config.manifest_path {
        command.arg("--manifest-path").arg(path);
    }

    for _ in 0..config.verbose {
        command.arg("-v");
    }

    if let Some(color) = &config.color {
        command.arg("--color").arg(color);
    }

    if config.frozen {
        command.arg("--frozen");
    }
    if config.locked {
        command.arg("--locked");
    }
    if config.offline {
        command.arg("--offline");
    }

    for flag in &config.unstable_flags {
        command.arg("-Z").arg(flag);
    }

    let output = output(&mut command, "cargo metadata")?;

    serde_json::from_str(&output).context("error parsing cargo metadata output")
}

fn default_target() -> Result<String, Error> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let output = output(Command::new(rustc).arg("-Vv"), "rustc")?;

    for line in output.lines() {
        let prefix = "host: ";
        if let Some(stripped_line) = line.strip_prefix(prefix) {
            return Ok(stripped_line.trim().to_string());
        }
    }

    Err(anyhow!("host missing from rustc output"))
}

fn output(command: &mut Command, job: &str) -> Result<String, Error> {
    let output = command
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("error running {}", job))?;

    if !output.status.success() {
        return Err(anyhow!("{} returned {}", job, output.status));
    }

    String::from_utf8(output.stdout).with_context(|| format!("error parsing {} output", job))
}
