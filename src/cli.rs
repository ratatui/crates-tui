use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, NoneAsEmptyString};
use tracing::level_filters::LevelFilter;

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

pub fn version() -> String {
    let author = clap::crate_authors!();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

"
    )
}

/// Command line arguments.
///
/// Implements Serialize so that we can use it as a source for Figment
/// configuration.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Parser, Serialize)]
#[command(author, version = version(), about, long_about = None)]
pub struct Cli {
    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 1.0)]
    pub tick_rate: f64,

    /// Print default configuration
    #[arg(long)]
    pub print_default_config: bool,

    /// A path to a crates-tui configuration file.
    #[arg(short, long, value_name = "FILE")]
    pub config_file: Option<PathBuf>,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 15.0)]
    pub frame_rate: f64,

    /// The directory to use for storing application data.
    #[arg(long, value_name = "DIR")]
    pub data_dir: Option<PathBuf>,

    /// The log level to use.
    ///
    /// Valid values are: error, warn, info, debug, trace, off. The default is
    /// info.
    #[arg(long, value_name = "LEVEL", alias = "log")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub log_level: Option<LevelFilter>,
}

// FIXME: seeing Cli::parse is pretty common and evokes that this is a clap
// parser, but cli::get slaps that expectation in the face, just enough to be
// annoying. Just let the caller call the function parse, it's not that big of a
// deal.
pub fn parse() -> Cli {
    Cli::parse()
}
