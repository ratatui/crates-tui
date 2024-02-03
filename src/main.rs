#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![feature(iter_intersperse)]

pub mod action;
pub mod app;
pub mod cli;
pub mod config;
pub mod mode;
pub mod picker;
pub mod tui;
pub mod utils;

use clap::Parser;
use cli::Cli;
use color_eyre::eyre::Result;
use config::initialize_config;

use crate::{
  app::App,
  utils::{initialize_logging, initialize_panic_handler, version},
};

async fn tokio_main() -> Result<()> {
  initialize_config()?;
  initialize_logging()?;
  initialize_panic_handler()?;

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
