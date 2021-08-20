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

/// Returns the name of the compose file in the supplied, if it exists.
#[instrument]
fn get_compose_file(path: &Path) -> Result<Option<Cow<'static, str>>> {
    if path.is_dir() {
        for entry in read_dir(path)? {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    for x in VALID_FILENAMES.iter() {
                        if x == &name {
                            return Ok(Some(Cow::Borrowed(x)));
                        }
                    }
                    for x in RECURSE_INTO_DIRS.iter() {
                        if x == &name {
                            if let Ok(Some(file)) =
                                find_compose_file(entry.path(), SearchDepth::Limited(1))
                            {
                                return Ok(Some(Cow::Owned(
                                    file.into_os_string().into_string().unwrap(),
                                )));
                            }
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

/// Searches for a docker-compose file, starting in the supplied directory
/// and working upwards up to `max_depth` levels.
#[instrument]
fn find_compose_file(path: PathBuf, max_depth: SearchDepth) -> Result<Option<PathBuf>> {
    let mut current = path.clone();
    let mut depth = 0;
    loop {
        debug!("Searching in {}", current.to_str().unwrap());
        if let Ok(Some(filename)) = get_compose_file(&current) {
            current.push(filename.as_ref());
            return Ok(Some(current));
        }

        depth += 1;
        match max_depth {
            SearchDepth::Limited(x) if depth >= x => return Ok(None),
            _ => {}
        }
        match current.parent() {
            None => return Ok(None),
            Some(x) => current = x.to_path_buf(),
        }
    }
}

/// Runs the supplied docker-compose command with the '-f' flag included.
#[instrument]
fn run_command(args: ArgsOs) -> Result<Child> {
    let compose_file = find_compose_file(current_dir()?, SearchDepth::Unlimited)?.ok_or(
        eyre!(
        "Couldn't find a docker-compose.yml or docker-compose.yaml file in any parent directory!"
    )
        .suggestion("Make sure you're in a project with a docker-compose file."),
    )?;
    Ok(Command::new("docker-compose")
        .arg("-f")
        .arg(compose_file)
        .args(args.skip(1))
        .spawn()?)
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