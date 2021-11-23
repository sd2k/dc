use std::{
    borrow::Cow,
    env::{args_os, current_dir, ArgsOs},
    fs::read_dir,
    path::{Path, PathBuf},
    process::{Child, Command},
};

use color_eyre::{eyre::eyre, Help, Result};
use tracing::{debug, instrument};
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

const RECURSE_INTO_DIRS: [&str; 1] = ["docker-compose"];
const VALID_FILENAMES: [&str; 2] = ["docker-compose.yml", "docker-compose.yaml"];
const VALID_OVERRIDE_FILENAMES: [&str; 2] = [
    "docker-compose.override.yml",
    "docker-compose.override.yaml",
];

/// Returns the name of the compose file in the supplied, if it exists.
#[instrument]
fn get_compose_file(
    path: &Path,
    valid_filenames: &'static [&'static str],
) -> Result<Option<Cow<'static, str>>> {
    if path.is_dir() {
        for entry in read_dir(path)?.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                for x in valid_filenames {
                    if x == &name {
                        return Ok(Some(Cow::Borrowed(x)));
                    }
                }
                for x in RECURSE_INTO_DIRS {
                    if x == name {
                        if let Some(file) = find_compose_file(
                            entry.path(),
                            SearchDepth::Limited(1),
                            valid_filenames,
                        ) {
                            return Ok(Some(Cow::Owned(
                                file.into_os_string().into_string().unwrap(),
                            )));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

/// The depth to search for a docker-compose.yaml, or docker-compose directory.
#[derive(Clone, Debug)]
enum SearchDepth {
    /// No limit on the search depth; searches will continue until the root directory.
    Unlimited,
    /// Only search up to the specified number of levels.
    Limited(usize),
}

impl SearchDepth {
    const fn is_exceeded(&self, depth: usize) -> bool {
        matches!(self, SearchDepth::Limited(x) if depth >= *x)
    }
}

/// Searches for a docker-compose file, starting in the supplied directory
/// and working upwards up to `max_depth` levels.
#[instrument]
fn find_compose_file(
    mut path: PathBuf,
    max_depth: SearchDepth,
    valid_filenames: &'static [&'static str],
) -> Option<PathBuf> {
    let mut depth = 0;
    loop {
        debug!("Searching in {}", path.to_str().unwrap());
        if let Ok(Some(filename)) = get_compose_file(&path, valid_filenames) {
            path.push(filename.as_ref());
            return Some(path);
        }

        depth += 1;
        if max_depth.is_exceeded(depth) {
            return None;
        }
        match path.parent() {
            None => return None,
            Some(x) => path = x.to_path_buf(),
        }
    }
}

/// Runs the supplied docker-compose command with the '-f' flag included.
#[instrument]
fn run_command(args: ArgsOs) -> Result<Child> {
    let compose_file = find_compose_file(current_dir()?, SearchDepth::Unlimited, &VALID_FILENAMES)
        .ok_or_else(|| {
            eyre!(
        "Couldn't find a docker-compose.yml or docker-compose.yaml file in any parent directory!"
    )
            .suggestion("Make sure you're in a project with a docker-compose file.")
        })?;
    let override_file = find_compose_file(
        current_dir()?,
        SearchDepth::Unlimited,
        &VALID_OVERRIDE_FILENAMES,
    );
    let mut cmd = Command::new("docker-compose");
    cmd.arg("-f").arg(compose_file);
    if let Some(f) = override_file {
        cmd.arg("-f").arg(f);
    }
    cmd.args(args.skip(1));
    debug!(command = ?cmd, "Running command");
    Ok(cmd.spawn()?)
}

fn install_tracing() {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

fn main() -> Result<()> {
    install_tracing();
    color_eyre::install()?;
    std::process::exit(run_command(args_os())?.wait()?.code().unwrap_or(-1));
}
