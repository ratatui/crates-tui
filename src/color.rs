use std::{collections::BTreeMap, str::FromStr};

use ratatui::prelude::*;
use ratatui_macros::palette;

palette!(pub SLATE);
palette!(pub GRAY);
palette!(pub ZINC);
palette!(pub NEUTRAL);
palette!(pub STONE);
palette!(pub RED);
palette!(pub ORANGE);
palette!(pub AMBER);
palette!(pub YELLOW);
palette!(pub LIME);
palette!(pub GREEN);
palette!(pub EMERALD);
palette!(pub TEAL);
palette!(pub CYAN);
palette!(pub SKY);
palette!(pub BLUE);
palette!(pub INDIGO);
palette!(pub VIOLET);
palette!(pub PURPLE);
palette!(pub FUCHSIA);
palette!(pub PINK);
palette!(pub ROSE);

struct Palette {
  base: Color,
  surface: Color,
  overlay: Color,
  muted: Color,
  subtle: Color,
  text: Color,
  love: Color,
  gold: Color,
  rose: Color,
  pine: Color,
  foam: Color,
  iris: Color,
  highlightlow: Color,
  highlightmed: Color,
  highlighthigh: Color,
}

impl Default for Palette {
  fn default() -> Self {
    Self {
      base: Color::from_str("#191724").unwrap(),
      surface: Color::from_str("#1F1D2E").unwrap(),
      overlay: Color::from_str("#26233A").unwrap(),
      muted: Color::from_str("#6E6A86").unwrap(),
      subtle: Color::from_str("#908CAA").unwrap(),
      text: Color::from_str("#E0DEF4").unwrap(),
      love: Color::from_str("#EB6F92").unwrap(),
      gold: Color::from_str("#F6C177").unwrap(),
      rose: Color::from_str("#EBBCBA").unwrap(),
      pine: Color::from_str("#31748F").unwrap(),
      foam: Color::from_str("#9CCFD8").unwrap(),
      iris: Color::from_str("#C4A7E7").unwrap(),
      highlightlow: Color::from_str("#21202E").unwrap(),
      highlightmed: Color::from_str("#403D52").unwrap(),
      highlighthigh: Color::from_str("#524F67").unwrap(),
    }
  }
}
