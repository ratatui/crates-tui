use std::{path::PathBuf, sync::OnceLock};

use color_eyre::eyre::{eyre, Result};
use directories::ProjectDirs;
use figment::{
  providers::{Env, Format, Serialized, Toml},
  Figment,
};
use ratatui::style::palette::tailwind::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, NoneAsEmptyString};
use tracing::level_filters::LevelFilter;

use crate::{cli::Cli, serde_helper::keybindings::KeyBindings};

static CONFIG: OnceLock<Config> = OnceLock::new();
pub const CONFIG_DEFAULT: &str = include_str!("../.config/config.default.toml");

/// Application configuration.
///
/// This is the main configuration struct for the application.
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
  /// The directory to use for storing application data (logs etc.).
  pub data_home: PathBuf,

  /// The directory to use for storing application configuration (colors etc.).
  pub config_home: PathBuf,

  /// The directory to use for storing application configuration (colors etc.).
  pub config_file: PathBuf,

  /// The log level to use. Valid values are: error, warn, info, debug, trace, off. The default is
  /// info.
  #[serde_as(as = "NoneAsEmptyString")]
  pub log_level: Option<LevelFilter>,

  pub tick_rate: f64,

  pub frame_rate: f64,

  pub key_refresh_rate: f64,

  pub enable_mouse: bool,

  pub enable_paste: bool,

  pub prompt_padding: u16,

  pub style: Style,

  pub key_bindings: KeyBindings,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Style {
  #[serde_as(as = "DisplayFromStr")]
  pub background_color: ratatui::style::Color,

  #[serde_as(as = "DisplayFromStr")]
  pub search_query_outline_color: ratatui::style::Color,

  #[serde_as(as = "DisplayFromStr")]
  pub filter_query_outline_color: ratatui::style::Color,

  #[serde_as(as = "DisplayFromStr")]
  pub row_background_color_1: ratatui::style::Color,

  #[serde_as(as = "DisplayFromStr")]
  pub row_background_color_2: ratatui::style::Color,
}

impl Default for Config {
  fn default() -> Self {
    let key_bindings: KeyBindings = Default::default();

    Self {
      data_home: default_data_dir(),
      config_home: default_config_dir(),
      config_file: default_config_file(),
      log_level: None,
      tick_rate: 1.0,
      frame_rate: 15.0,
      key_refresh_rate: 0.5,
      enable_mouse: false,
      enable_paste: false,
      prompt_padding: 1,
      key_bindings,
      style: Style {
        background_color: GRAY.c900,
        search_query_outline_color: GREEN.c400,
        filter_query_outline_color: YELLOW.c400,
        row_background_color_1: GRAY.c900,
        row_background_color_2: GRAY.c800,
      },
    }
  }
}

/// Returns the directory to use for storing data files.
fn default_data_dir() -> PathBuf {
  if let Some(dir) =
    std::env::var(format!("{}_DATA_HOME", env!("CARGO_CRATE_NAME").to_uppercase())).ok().map(PathBuf::from)
  {
    dir
  } else if let Ok(dir) = project_dirs().map(|dirs| dirs.data_local_dir().to_path_buf()) {
    dir
  } else {
    PathBuf::from(".").join(".data")
  }
}

/// Returns the directory to use for storing config files.
fn default_config_dir() -> PathBuf {
  if let Some(dir) =
    std::env::var(format!("{}_CONFIG_HOME", env!("CARGO_CRATE_NAME").to_uppercase())).ok().map(PathBuf::from)
  {
    dir
  } else if let Ok(dir) = project_dirs().map(|dirs| dirs.config_local_dir().to_path_buf()) {
    dir
  } else {
    PathBuf::from(".").join(".config")
  }
}

/// Returns the path to the default configuration file.
fn default_config_file() -> PathBuf {
  default_config_dir().join("config.toml")
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
pub fn initialize_config(cli: &Cli) -> Result<()> {
  let config_file = cli.config_file.clone().unwrap_or_else(default_config_file);
  let config = Figment::new()
    .merge(Serialized::defaults(Config::default()))
    .merge(Toml::string(CONFIG_DEFAULT))
    .merge(Toml::file(config_file))
    .merge(Env::prefixed(concat!(env!("CARGO_CRATE_NAME"), "_")))
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
