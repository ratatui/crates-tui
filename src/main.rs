mod action;
mod app;
mod cli;
mod command;
mod config;
mod crates_io_api_helper;
mod errors;
mod events;
mod logging;
mod serde_helper;
mod widgets;

use app::App;
use color_eyre::eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::parse();
    config::init(&cli)?;
    logging::init()?;
    errors::install_hooks()?;

    if cli.print_default_config {
        println!("{}", toml::to_string_pretty(config::get())?);
        return Ok(());
    }

    let events = events::Events::new();
    let runtime = tokio::runtime::Handle::current();
    ratatui::run(move |tui| App::new().run(tui, runtime, events, cli.query))
}
