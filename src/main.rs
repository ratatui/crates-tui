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

fn main() -> color_eyre::Result<()> {
    let cli = cli::parse();
    config::init(&cli)?;
    logging::init()?;
    errors::install_hooks()?;

    if cli.print_default_config {
        println!("{}", toml::to_string_pretty(config::get())?);
        return Ok(());
    }

    let mut app = App::new(cli.query);
    ratatui::run(|tui| app.run(tui))
}
