# crates-tui

A TUI viewer for crates.io

https://github.com/ratatui-org/crates-tui/assets/1813121/c84eaad7-4688-4ebb-91c0-683cc9a0abfe

## Widgets

```plain
                                                                                           █
   Name                 Description                         Downloads  Last Updated        █
                                                                                           █
                       │                                  │          │                     █
 • ratatui             │A library that's all about cooking│ 925,193  │ 2024-02-03 00:14:47 █
                       │up terminal user interfaces       │          │
                       │                                  │          │
                       │                                  │          │
   ansi-to-tui         │A library to convert ansi         │ 82,241   │ 2023-06-23 06:08:04
                       │color coded text into             │          │
                       │ratatui::text::Text type from     │          │
                       │ratatui library                   │          │
                       │                                  │          │
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│Name               ratatui                                                                │
│Created At         2023-02-08 17:11:50                                                    │
│Updated At         2023-02-08 17:11:50                                                    │
│Max Version        0.26.1-alpha.0                                                         │
│Description        A library that's all about cooking up terminal user interfaces         │
│Repository         https://github.com/ratatui-org/ratatui                                 │
│Recent Downloads   448198                                                                 │
│Max Stable Version 0.26.0                                                                 │
│                                                                                          │
│                                                                                          │
└──────────────────────────────────────────────────────────────────────────────────────────┘
┌Query (Press ? to search, / to filter, Enter to submit)───────────────────────────────1/93┐
│                                                                                          │
│ ratatui                                                                                  │
│                                                                                          │
└─────────────────────────────────────────────────────────────────────────────────["g", "g"]
```

**Crates Table**

- Rows are of different height based on description text wrapping
- Column spacers are filled in with a separator
- Scrollbar on the right

**Crates Info**

- Crate Info shown in a table that can be toggled on and off

**Prompt**

- Allows user to search or filter
- Changes color of border depending on whether query is search or filter
- Supports readline shortcuts
- Shows table state at the top right
- Shows user key presses per tick in bottom right

## Logging

https://github.com/ratatui-org/crates-tui/assets/1813121/9609a0f1-4da7-426d-8ce8-2c5a77c54754

## Print Default Configuration

```plain
$ crates-tui --print-default-config

Config {
    data_home: "",
    config_home: "",
    log_level: Some(
        LevelFilter::DEBUG,
    ),
    tick_rate: 1.0,
    frame_rate: 15.0,
    background_color: Reset,
    search_query_outline_color: Reset,
    filter_query_outline_color: Reset,
    row_background_color_1: Reset,
    row_background_color_2: Reset,
}
```
