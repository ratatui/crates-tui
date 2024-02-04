pub mod action;
pub mod app;
pub mod cli;
pub mod config;
pub mod errors;
pub mod logging;
pub mod picker;
pub mod tui;

use color_eyre::eyre::Result;

use crate::{app::App, config::initialize_config, errors::initialize_panic_handler, logging::initialize_logging};

async fn tokio_main() -> Result<()> {
  let cli = cli::get();
  initialize_config(&cli)?;
  initialize_logging()?;
  initialize_panic_handler()?;

  if cli.print_config {
    println!("{:#?}", config::get());
    return Ok(());
  }

  let mut tui = tui::Tui::new()?.tick_rate(config::get().tick_rate).frame_rate(config::get().frame_rate);
  let mut app = App::new()?;
  app.run(&mut tui).await?;

  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  if let Err(e) = tokio_main().await {
    eprintln!("{} error: Something went wrong", env!("CARGO_PKG_NAME"));
    Err(e)
  } else {
    Ok(())
  }
}
