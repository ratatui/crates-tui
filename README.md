# crates-tui

`crates-tui` is a simple TUI explorer for crates.io from the terminal. It supports features like 

- copy `cargo add` command to clipboard
- open the docs page in the browser
- open crates.io page in the brower

<img width="350" alt="image" src="https://github.com/ratatui-org/crates-tui/assets/1813121/62d9234f-59a8-4091-ba50-7cd050d9763a">
<img width="350" alt="image" src="https://github.com/ratatui-org/crates-tui/assets/1813121/e12a3320-1232-46e1-951e-14c9d20f0734">
<img width="350" alt="image" src="https://github.com/ratatui-org/crates-tui/assets/1813121/21fcbf12-63c1-4952-aa5e-1d926f4919a0">
<img width="350" alt="image" src="https://github.com/ratatui-org/crates-tui/assets/1813121/25e8eca1-68bf-4560-a55f-0a4b7fcebe81">

## Install

```rust
cargo install crates-tui
```

## Screenshots

### Open in browser

https://github.com/ratatui-org/crates-tui/assets/1813121/362d7dc3-d9ef-43df-8d2e-cc56001ef31c

### Logging

https://github.com/ratatui-org/crates-tui/assets/1813121/9609a0f1-4da7-426d-8ce8-2c5a77c54754

### Base16 Theme

[**Dracula**](https://github.com/dracula/base16-dracula-scheme/blob/master/dracula.yaml)

<img width="750" alt="image" src="https://github.com/ratatui-org/crates-tui/assets/1813121/0c65b9a2-cc01-4c40-bf3e-79f6522411d8">

[**Rose Pine**](https://github.com/edunfelt/base16-rose-pine-scheme)

<img width="750" alt="image" src="https://github.com/ratatui-org/crates-tui/assets/1813121/5130a654-76c0-411b-8fbb-5ea9946acdd7">

[**GitHub**](https://github.com/Defman21/base16-github-scheme)

<img width="748" alt="image" src="https://github.com/ratatui-org/crates-tui/assets/1813121/8f6d5ede-b0c6-418c-9762-41964a9dcee6">

### Help

https://github.com/ratatui-org/crates-tui/assets/1813121/4c2a3deb-f546-41e6-a48d-998831182ab6

### Key to Action configurations per mode

Here's the default configuration

```toml
tick_rate = 1.0
frame_rate = 15.0
key_refresh_rate = 0.5

[key_bindings.popup]
"j" = "ScrollDown"
"k" = "ScrollUp"
"Down" = "ScrollDown"
"Up" = "ScrollUp"
"Enter" = "ClosePopup"
"ESC" = "ClosePopup"

[key_bindings.picker_show_crate_info]
"q" = { SwitchMode = "picker_hide_crate_info" }
"ESC" = { SwitchMode = "picker_hide_crate_info" }
"[" = { SwitchMode = "summary" }
"<g><g>" = "ScrollTop"
"G" = "ScrollBottom"
"Home" = "ScrollTop"
"End" = "ScrollBottom"
"?" = { SwitchMode = "help" }
"/" = { SwitchMode = "search" }
"\\" = { SwitchMode = "filter" }
"j" = "ScrollDown"
"k" = "ScrollUp"
"ctrl-j" = "ScrollCrateInfoDown"
"ctrl-k" = "ScrollCrateInfoUp"
"TAB" = { ToggleSortBy = { reload = true, forward = true } }
"Shift-Backtab" = { ToggleSortBy = { reload = true, forward = false } }
"Down" = "ScrollDown"
"Up" = "ScrollUp"
"l" = "IncrementPage"
"h" = "DecrementPage"
"Left" = "DecrementPage"
"Right" = "IncrementPage"
"r" = "ReloadData"
"Enter" = "ToggleShowCrateInfo"
"<g><d>" = "OpenDocsUrlInBrowser"
"<g><c>" = "OpenCratesIOUrlInBrowser"
"c" = "CopyCargoAddCommandToClipboard"

[key_bindings.picker_hide_crate_info]
"q" = "Quit"
"ESC" = "Quit"
"[" = { SwitchMode = "summary" }
"<g><g>" = "ScrollTop"
"G" = "ScrollBottom"
"Home" = "ScrollTop"
"End" = "ScrollBottom"
"?" = { SwitchMode = "help" }
"/" = { SwitchMode = "search" }
"\\" = { SwitchMode = "filter" }
"j" = "ScrollDown"
"k" = "ScrollUp"
"ctrl-j" = "ScrollCrateInfoDown"
"ctrl-k" = "ScrollCrateInfoUp"
"TAB" = { ToggleSortBy = { reload = true, forward = true } }
"Shift-Backtab" = { ToggleSortBy = { reload = true, forward = false } }
"Down" = "ScrollDown"
"Up" = "ScrollUp"
"l" = "IncrementPage"
"h" = "DecrementPage"
"Left" = "DecrementPage"
"Right" = "IncrementPage"
"r" = "ReloadData"
"Enter" = "ToggleShowCrateInfo"
"<g><d>" = "OpenDocsUrlInBrowser"
"<g><c>" = "OpenCratesIOUrlInBrowser"
"<g><Enter>" = "ShowFullCrateInfo"
"c" = "CopyCargoAddCommandToClipboard"

[key_bindings.summary]
"]" = { SwitchMode = "picker_hide_crate_info" }
"q" = "Quit"
"ESC" = "Quit"
"j" = "ScrollDown"
"k" = "ScrollUp"
"h" = "PreviousSummaryMode"
"l" = "NextSummaryMode"
"?" = { SwitchMode = "help" }
"/" = { SwitchMode = "search" }
"\\" = { SwitchMode = "filter" }

[key_bindings.help]
"q" = "Quit"
"ESC" = "SwitchToLastMode"
"j" = "ScrollDown"
"k" = "ScrollUp"

[key_bindings.search]
"TAB" = { ToggleSortBy = { reload = false, forward = true } }
"Shift-Backtab" = { ToggleSortBy = { reload = false, forward = false } }
"ESC" = { SwitchMode = "picker_hide_crate_info" }
"Enter" = "SubmitSearch"
"ctrl-j" = "ScrollSearchResultsDown"
"ctrl-k" = "ScrollSearchResultsUp"

[key_bindings.filter]
"ESC" = { SwitchMode = "picker_hide_crate_info" }
"Enter" = { SwitchMode = "picker_hide_crate_info" }
"ctrl-j" = "ScrollSearchResultsDown"
"ctrl-k" = "ScrollSearchResultsUp"
```

## Background

This repository contains an opinionated way of organizing a small to medium sized Ratatui TUI
applications.

It has several features, notably:

- Uses `async` to fetch crate information without blocking the UI
- Multiple custom widgets
  - Selection tab
  - Input prompt
  - Search results table
  - Summary view
- Has configurable key chords that map to actions

This repository is meant to serve as a reference for some patterns you may follow when developing
Ratatui applications. The code will function as a reference for the tutorial material on
https://ratatui.rs as well.
