use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Display, Deserialize)]
pub enum Action {
  Tick,
  Render,
  Resize(u16, u16),
  Suspend,
  Resume,
  Quit,
  Refresh,
  Error(String),
  CloseError,
  Help,
  GetCrates,
  EnterSearchQueryInsert,
  EnterFilterInsert,
  IncrementPage,
  DecrementPage,
  EnterNormal,
  MoveSelectionBottom,
  MoveSelectionTop,
  MoveSelectionNext,
  MoveSelectionPrevious,
  SubmitSearchQuery,
  GetInfo,
  ReloadData,
  ToggleShowHelp,
  ToggleShowCrateInfo,
  StoreTotalNumberOfCrates(u64),
  CargoAddCrate,
  ShowCargoAddCompletionMessage(String),
}
