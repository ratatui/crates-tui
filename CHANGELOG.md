# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6](https://github.com/ratatui-org/crates-tui/compare/v0.1.5...v0.1.6) - 2024-02-11

### Added
- refactor app.rs ([#21](https://github.com/ratatui-org/crates-tui/pull/21))
- Increase spacing between top and bottom in summary

### Fixed
- version string even without git ([#24](https://github.com/ratatui-org/crates-tui/pull/24))

### Other
- Add AUR instructions ([#22](https://github.com/ratatui-org/crates-tui/pull/22))

## [0.1.5](https://github.com/ratatui-org/crates-tui/compare/v0.1.4...v0.1.5) - 2024-02-09

### Added
- Show cargo copy in demo
- Show help in demo
- Add vhs tape
- Better help menu with offset and UX for new users
- Add Action::Ignore

### Fixed
- Don't update crate info when scrolling help
- Change resolution in tape
- Missing enter in vhs tape

### Other
- Update gitignore to only ignore log files
- Update README.md with new demo
- more tweaks to the demo by scrolling help
- Tweak resolution and timing of the demo
- Increase resolution of demo
- Change demo to move up faster at the end

## [0.1.4](https://github.com/ratatui-org/crates-tui/compare/v0.1.3...v0.1.4) - 2024-02-09

### Fixed
- version string

## [0.1.3](https://github.com/ratatui-org/crates-tui/compare/v0.1.2...v0.1.3) - 2024-02-09

### Other
- Remove musl automated build

## [0.1.2](https://github.com/ratatui-org/crates-tui/compare/v0.1.1...v0.1.2) - 2024-02-09

### Other
- Add token to checkout

## [0.1.1](https://github.com/ratatui-org/crates-tui/compare/v0.1.0...v0.1.1) - 2024-02-09

### Added

- Open crates io pages from summary view
- Color theme support and configurable colors
- Better popup scroll
- Add copy cargo add command to clipboard
- Always show spinner in top right
- Add page number
- Better prompt
- Add summary screen
- Only show keywords instead of versions

### Fixed

- Popup scroll bug

### Other

- simplify popup ([#12](https://github.com/ratatui-org/crates-tui/pull/12))
- better keybinds ([#11](https://github.com/ratatui-org/crates-tui/pull/11))
- use cfg_if crate for better cfg checks ([#9](https://github.com/ratatui-org/crates-tui/pull/9))
- move events from tui to events module ([#8](https://github.com/ratatui-org/crates-tui/pull/8))
- simplify tui, events, errors ([#7](https://github.com/ratatui-org/crates-tui/pull/7))
- cleanup config.rs ([#6](https://github.com/ratatui-org/crates-tui/pull/6))
