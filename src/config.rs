use std::{path::PathBuf, sync::OnceLock};

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use directories::ProjectDirs;
use figment::{
  providers::{Env, Format, Serialized, Toml},
  Figment,
};
use ratatui::style::palette::tailwind::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr, NoneAsEmptyString};
use tracing::level_filters::LevelFilter;

use crate::utils::version;

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Command line arguments.
///
/// Implements Serialize so that we can use it as a source for Figment configuration.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Parser, Serialize)]
#[command(author, version = version(), about, long_about = None)]
pub struct Cli {
  #[arg(
    short,
    long,
    value_name = "FLOAT",
    help = "Tick rate, i.e. number of ticks per second",
    default_value_t = 10.0
  )]
  pub tick_rate: f64,

  /// A path to a crates-tui configuration file.
  #[arg(short, long, value_name = "FILE")]
  config: Option<PathBuf>,

  #[arg(
    short,
    long,
    value_name = "FLOAT",
    help = "Frame rate, i.e. number of frames per second",
    default_value_t = 15.0
  )]
  pub frame_rate: f64,

  /// The directory to use for storing application data.
  #[arg(long, value_name = "DIR")]
  pub data_dir: Option<PathBuf>,

  /// The log level to use.
  ///
  /// Valid values are: error, warn, info, debug, trace, off. The default is info.
  #[arg(long, value_name = "LEVEL", default_value = "info", alias = "log")]
  #[serde_as(as = "NoneAsEmptyString")]
  pub log_level: Option<LevelFilter>,
}

/// Application configuration.
///
/// This is the main configuration struct for the application.
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
  /// The directory to use for storing application data (logs etc.).
  pub data_dir: PathBuf,

  /// The log level to use. Valid values are: error, warn, info, debug, trace, off. The default is
  /// info.
  #[serde_as(as = "DisplayFromStr")]
  pub log_level: LevelFilter,

  pub tick_rate: f64,

  pub frame_rate: f64,

  #[serde_as(as = "DisplayFromStr")]
  pub background_color: ratatui::style::Color,

  #[serde_as(as = "DisplayFromStr")]
  pub search_query_outline_color: ratatui::style::Color,

  #[serde_as(as = "DisplayFromStr")]
  pub filter_query_outline_color: ratatui::style::Color,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      data_dir: default_data_dir(),
      log_level: LevelFilter::INFO,
      tick_rate: 1.0,
      frame_rate: 4.0,
      background_color: GRAY.c900,
      search_query_outline_color: GREEN.c400,
      filter_query_outline_color: GREEN.c400,
    }
  }
}

/// Returns the directory to use for storing data files.
fn default_data_dir() -> PathBuf {
  project_dirs().map(|dirs| dirs.data_local_dir().to_path_buf()).unwrap()
}

/// Returns the path to the default configuration file.
fn default_config_file() -> PathBuf {
  project_dirs().map(|dirs| dirs.config_local_dir().join("config.toml")).unwrap()
}

/// Returns the project directories.
fn project_dirs() -> Result<ProjectDirs> {
  ProjectDirs::from("com", "kdheepak", env!("CARGO_PKG_NAME")).ok_or_else(|| eyre!("user home directory not found"))
}

/// Initialize the application configuration.
///
/// This function should be called before any other function in the application.
/// It will initialize the application config from the following sources:
/// - default values
/// - a configuration file
/// - environment variables
/// - command line arguments
pub fn initialize_config() -> Result<()> {
  let cli = Cli::parse();
  let config_file = cli.config.clone().unwrap_or_else(default_config_file);
  let config = Figment::new()
    .merge(Serialized::defaults(Config::default()))
    .merge(Toml::file(config_file))
    .merge(Env::prefixed("CRATES_TUI_"))
    .merge(Serialized::defaults(cli))
    .extract::<Config>()?;
  CONFIG.set(config).map_err(|config| eyre!("failed to set config {config:?}"))
}

/// Get the application configuration.
///
/// This function should only be called after [`init()`] has been called.
///
/// # Panics
///
/// This function will panic if [`init()`] has not been called.
pub fn get() -> &'static Config {
  CONFIG.get().expect("config not initialized")
}
