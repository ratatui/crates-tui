use serde::{Deserialize, Serialize};
use strum::Display;

use crate::app::Mode;

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Command {
    Ignore,
    Quit,
    NextTab,
    PreviousTab,
    ClosePopup,
    SwitchMode(Mode),
    SwitchToLastMode,
    IncrementPage,
    DecrementPage,
    NextSummaryMode,
    PreviousSummaryMode,
    ToggleSortBy { reload: bool, forward: bool },
    ScrollBottom,
    ScrollTop,
    ScrollDown,
    ScrollUp,
    ScrollCrateInfoDown,
    ScrollCrateInfoUp,
    ScrollSearchResultsDown,
    ScrollSearchResultsUp,
    SubmitSearch,
    ReloadData,
    ToggleShowCrateInfo,
    CopyCargoAddCommandToClipboard,
    OpenDocsUrlInBrowser,
    OpenCratesIOUrlInBrowser,
}
