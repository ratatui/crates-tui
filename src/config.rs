use std::{env, path::PathBuf, str::FromStr, sync::OnceLock};

use color_eyre::eyre::{eyre, Result};
use directories::ProjectDirs;
use figment::{
    providers::{Env, Format, Serialized, Toml, Yaml},
    Figment,
};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, NoneAsEmptyString};
use tracing::level_filters::LevelFilter;

use crate::{cli::Cli, serde_helper::keybindings::KeyBindings};

static CONFIG: OnceLock<Config> = OnceLock::new();
pub const CONFIG_DEFAULT: &str = include_str!("../.config/config.default.toml");

#[serde_as]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Base16Palette {
    /// Default Background
    #[serde_as(as = "DisplayFromStr")]
    pub base00: Color,

    /// Lighter Background (Used for status bars, line number and folding marks)
    #[serde_as(as = "DisplayFromStr")]
    pub base01: Color,

    /// Selection Background (Settings where you need to highlight text, such as find results)
    #[serde_as(as = "DisplayFromStr")]
    pub base02: Color,

    /// Comments, Invisibles, Line Highlighting
    #[serde_as(as = "DisplayFromStr")]
    pub base03: Color,

    /// Dark Foreground (Used for status bars)
    #[serde_as(as = "DisplayFromStr")]
    pub base04: Color,

    /// Default Foreground, Caret, Delimiters, Operators
    #[serde_as(as = "DisplayFromStr")]
    pub base05: Color,

    /// Light Foreground (Not often used, could be used for hover states or dividers)
    #[serde_as(as = "DisplayFromStr")]
    pub base06: Color,

    /// Light Background (Probably at most for cursor line background color)
    #[serde_as(as = "DisplayFromStr")]
    pub base07: Color,

    /// Variables, XML Tags, Markup Link Text, Markup Lists, Diff Deleted
    #[serde_as(as = "DisplayFromStr")]
    pub base08: Color,

    /// Integers, Boolean, Constants, XML Attributes, Markup Link Url
    #[serde_as(as = "DisplayFromStr")]
    pub base09: Color,

    /// Classes, Keywords, Storage, Selector, Markup Italic, Diff Changed
    #[serde_as(as = "DisplayFromStr")]
    pub base0a: Color,

    /// Strings, Inherited Class, Markup Code, Diff Inserted
    #[serde_as(as = "DisplayFromStr")]
    pub base0b: Color,

    /// Support, Regular Expressions, Escape Characters, Markup Quotes
    #[serde_as(as = "DisplayFromStr")]
    pub base0c: Color,

    /// Functions, Methods, Attribute IDs, Headings
    #[serde_as(as = "DisplayFromStr")]
    pub base0d: Color,

    /// Keywords, Storage, Selector, Markup Bold, Diff Renamed
    #[serde_as(as = "DisplayFromStr")]
    pub base0e: Color,

    /// Deprecated, Opening/Closing Embedded Language Tags e.g., `<? ?>`
    #[serde_as(as = "DisplayFromStr")]
    pub base0f: Color,
}

impl Default for Base16Palette {
    fn default() -> Self {
        Self {
            base00: Color::from_str("#191724").unwrap(),
            base01: Color::from_str("#1f1d2e").unwrap(),
            base02: Color::from_str("#26233a").unwrap(),
            base03: Color::from_str("#6e6a86").unwrap(),
            base04: Color::from_str("#908caa").unwrap(),
            base05: Color::from_str("#e0def4").unwrap(),
            base06: Color::from_str("#e0def4").unwrap(),
            base07: Color::from_str("#524f67").unwrap(),
            base08: Color::from_str("#eb6f92").unwrap(),
            base09: Color::from_str("#f6c177").unwrap(),
            base0a: Color::from_str("#ebbcba").unwrap(),
            base0b: Color::from_str("#31748f").unwrap(),
            base0c: Color::from_str("#9ccfd8").unwrap(),
            base0d: Color::from_str("#c4a7e7").unwrap(),
            base0e: Color::from_str("#f6c177").unwrap(),
            base0f: Color::from_str("#524f67").unwrap(),
        }
    }
}

/// Application configuration.
///
/// This is the main configuration struct for the application.
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// The directory to use for storing application data (logs etc.).
    pub data_home: PathBuf,

    /// The directory to use for storing application configuration (colors
    /// etc.).
    pub config_home: PathBuf,

    /// The directory to use for storing application configuration (colors
    /// etc.).
    pub config_file: PathBuf,

    /// The log level to use. Valid values are: error, warn, info, debug, trace,
    /// off. The default is info.
    #[serde_as(as = "NoneAsEmptyString")]
    pub log_level: Option<LevelFilter>,

    pub tick_rate: f64,

    pub frame_rate: f64,

    pub key_refresh_rate: f64,

    pub enable_mouse: bool,

    pub enable_paste: bool,

    pub prompt_padding: u16,

    pub key_bindings: KeyBindings,

    pub color: Base16Palette,
}

impl Default for Config {
    fn default() -> Self {
        let key_bindings: KeyBindings = Default::default();
        let rose_pine = Base16Palette::default();

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
            color: rose_pine,
        }
    }
}

/// Initialize the application configuration.
///
/// This function should be called before any other function in the application.
/// It will initialize the application config from the following sources:
/// - default values
/// - a configuration file
/// - environment variables
/// - command line arguments
pub fn init(cli: &Cli) -> Result<()> {
    let config_file = cli.config_file.clone().unwrap_or_else(default_config_file);
    let color_file = cli.color_file.clone().unwrap_or_else(default_color_file);
    let mut config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::string(CONFIG_DEFAULT))
        .merge(Toml::file(config_file))
        .merge(Env::prefixed("CRATES_TUI_"))
        .merge(Serialized::defaults(cli))
        .extract::<Config>()?;
    let base16 = Figment::new()
        .merge(Serialized::defaults(Base16Palette::default()))
        .merge(Yaml::file(color_file))
        .extract::<Base16Palette>()?;
    config.color = base16;
    CONFIG
        .set(config)
        .map_err(|config| eyre!("failed to set config {config:?}"))
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

/// Returns the path to the default configuration file.
fn default_config_file() -> PathBuf {
    default_config_dir().join("config.toml")
}

/// Returns the path to the default configuration file.
fn default_color_file() -> PathBuf {
    default_config_dir().join("color.yaml")
}

/// Returns the directory to use for storing config files.
fn default_config_dir() -> PathBuf {
    env::var("CRATES_TUI_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| project_dirs().map(|dirs| dirs.config_local_dir().to_path_buf()))
        .unwrap_or(PathBuf::from(".").join(".config"))
}

/// Returns the directory to use for storing data files.
fn default_data_dir() -> PathBuf {
    env::var("CRATES_TUI_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|_| project_dirs().map(|dirs| dirs.data_local_dir().to_path_buf()))
        .unwrap_or(PathBuf::from(".").join(".data"))
}

/// Returns the project directories.
fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("rs", "ratatui", "crates-tui")
        .ok_or_else(|| eyre!("user home directory not found"))
}

#[cfg(test)]
mod tests {
    use crate::serde_helper::keybindings::parse_key_sequence;

    use super::*;

    #[test]

    fn create_config() {
        let mut c = Config::default();
        c.key_bindings.insert(
            crate::app::Mode::PickerShowCrateInfo,
            &parse_key_sequence("q").unwrap(),
            crate::action::Action::Quit,
        );

        println!("{}", toml::to_string_pretty(&c).unwrap());
    }
}
