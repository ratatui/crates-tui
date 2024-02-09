pub mod keybindings {
    use std::collections::HashMap;

    use color_eyre::eyre::Result;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use derive_deref::{Deref, DerefMut};
    use itertools::Itertools;
    use serde::{de::Deserializer, Deserialize, Serialize, Serializer};

    use crate::{action::Action, app::Mode, command::Command};

    #[derive(Clone, Debug, Default, Deref, DerefMut)]
    pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Command>>);

    impl KeyBindings {
        pub fn command_to_action(&self, command: Command) -> Action {
            match command {
                Command::Ignore => Action::Ignore,
                Command::Quit => Action::Quit,
                Command::NextTab => Action::NextTab,
                Command::PreviousTab => Action::PreviousTab,
                Command::ClosePopup => Action::ClosePopup,
                Command::SwitchMode(m) => Action::SwitchMode(m),
                Command::SwitchToLastMode => Action::SwitchToLastMode,
                Command::IncrementPage => Action::IncrementPage,
                Command::DecrementPage => Action::DecrementPage,
                Command::NextSummaryMode => Action::NextSummaryMode,
                Command::PreviousSummaryMode => Action::PreviousSummaryMode,
                Command::ToggleSortBy { reload, forward } => {
                    Action::ToggleSortBy { reload, forward }
                }
                Command::ScrollBottom => Action::ScrollBottom,
                Command::ScrollTop => Action::ScrollTop,
                Command::ScrollDown => Action::ScrollDown,
                Command::ScrollUp => Action::ScrollUp,
                Command::ScrollCrateInfoDown => Action::ScrollCrateInfoDown,
                Command::ScrollCrateInfoUp => Action::ScrollCrateInfoUp,
                Command::ScrollSearchResultsDown => Action::ScrollSearchResultsDown,
                Command::ScrollSearchResultsUp => Action::ScrollSearchResultsUp,
                Command::SubmitSearch => Action::SubmitSearch,
                Command::ReloadData => Action::ReloadData,
                Command::ToggleShowCrateInfo => Action::ToggleShowCrateInfo,
                Command::CopyCargoAddCommandToClipboard => Action::CopyCargoAddCommandToClipboard,
                Command::OpenDocsUrlInBrowser => Action::OpenDocsUrlInBrowser,
                Command::OpenCratesIOUrlInBrowser => Action::OpenCratesIOUrlInBrowser,
            }
        }

        #[allow(dead_code)]
        pub fn insert(&mut self, mode: Mode, key_events: &[KeyEvent], command: Command) {
            // Convert the slice of `KeyEvent`(s) to a `Vec`.
            let key_events_vec = key_events.to_vec();

            // Retrieve or create the inner `HashMap` corresponding to the mode.
            let bindings_for_mode = self.0.entry(mode).or_default();

            // Insert the `Command` into the inner `HashMap` using the key events `Vec` as
            // the key.
            bindings_for_mode.insert(key_events_vec, command);
        }

        pub fn event_to_command(&self, mode: Mode, key_events: &[KeyEvent]) -> Option<Command> {
            if key_events.is_empty() {
                None
            } else if let Some(Some(command)) = self.0.get(&mode).map(|kb| kb.get(key_events)) {
                Some(command.clone())
            } else {
                self.event_to_command(mode, &key_events[1..])
            }
        }

        pub fn get_keybindings_for_command(
            &self,
            mode: Mode,
            command: Command,
        ) -> Vec<Vec<KeyEvent>> {
            let bindings_for_mode = self.0.get(&mode).cloned().unwrap_or_default();
            bindings_for_mode
                .into_iter()
                .filter(|(_, v)| *v == command)
                .map(|(k, _)| k)
                .collect_vec()
        }

        pub fn get_config_for_command(&self, mode: Mode, command: Command) -> Vec<String> {
            self.get_keybindings_for_command(mode, command)
                .iter()
                .map(|key_events| {
                    key_events
                        .iter()
                        .map(key_event_to_string)
                        .collect_vec()
                        .join("")
                })
                .collect_vec()
        }
    }

    impl<'de> Deserialize<'de> for KeyBindings {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let parsed_map = HashMap::<Mode, HashMap<String, Command>>::deserialize(deserializer)?;

            let keybindings = parsed_map
                .into_iter()
                .map(|(mode, inner_map)| {
                    let converted_inner_map = inner_map
                        .into_iter()
                        .map(|(key_str, cmd)| (parse_key_sequence(&key_str).unwrap(), cmd))
                        .collect();
                    (mode, converted_inner_map)
                })
                .collect();

            Ok(KeyBindings(keybindings))
        }
    }

    impl Serialize for KeyBindings {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut serialized_map: HashMap<Mode, HashMap<String, Command>> = HashMap::new();

            for (mode, key_event_map) in self.0.iter() {
                let mut string_event_map = HashMap::new();

                for (key_events, command) in key_event_map {
                    let key_string = key_events
                        .iter()
                        .map(|key_event| format!("<{}>", key_event_to_string(key_event)))
                        .collect::<Vec<String>>()
                        .join("");

                    string_event_map.insert(key_string, command.clone());
                }

                serialized_map.insert(*mode, string_event_map);
            }

            serialized_map.serialize(serializer)
        }
    }

    fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
        let (remaining, modifiers) = extract_modifiers(raw);
        parse_key_code_with_modifiers(remaining, modifiers)
    }

    fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
        let mut modifiers = KeyModifiers::empty();
        let mut current = raw;

        loop {
            match current {
                rest if rest.to_lowercase().starts_with("ctrl-") => {
                    modifiers.insert(KeyModifiers::CONTROL);
                    current = &rest[5..];
                }
                rest if rest.to_lowercase().starts_with("alt-") => {
                    modifiers.insert(KeyModifiers::ALT);
                    current = &rest[4..];
                }
                rest if rest.to_lowercase().starts_with("shift-") => {
                    modifiers.insert(KeyModifiers::SHIFT);
                    current = &rest[6..];
                }
                _ => break, // break out of the loop if no known prefix is detected
            };
        }

        (current, modifiers)
    }

    // FIXME - seems excessively verbose. Use strum to simplify?
    fn parse_key_code_with_modifiers(
        raw: &str,
        mut modifiers: KeyModifiers,
    ) -> Result<KeyEvent, String> {
        let c = match raw.to_lowercase().as_str() {
            "esc" => KeyCode::Esc,
            "enter" => KeyCode::Enter,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "backtab" => {
                modifiers.insert(KeyModifiers::SHIFT);
                KeyCode::BackTab
            }
            "backspace" => KeyCode::Backspace,
            "delete" => KeyCode::Delete,
            "insert" => KeyCode::Insert,
            "f1" => KeyCode::F(1),
            "f2" => KeyCode::F(2),
            "f3" => KeyCode::F(3),
            "f4" => KeyCode::F(4),
            "f5" => KeyCode::F(5),
            "f6" => KeyCode::F(6),
            "f7" => KeyCode::F(7),
            "f8" => KeyCode::F(8),
            "f9" => KeyCode::F(9),
            "f10" => KeyCode::F(10),
            "f11" => KeyCode::F(11),
            "f12" => KeyCode::F(12),
            "space" => KeyCode::Char(' '),
            "hyphen" => KeyCode::Char('-'),
            "minus" => KeyCode::Char('-'),
            "tab" => KeyCode::Tab,
            c if c.len() == 1 => {
                let mut c = raw.chars().next().unwrap();
                if modifiers.contains(KeyModifiers::SHIFT) {
                    c = c.to_ascii_uppercase();
                }
                KeyCode::Char(c)
            }
            _ => return Err(format!("Unable to parse {raw}")),
        };
        Ok(KeyEvent::new(c, modifiers))
    }

    pub fn key_event_to_string(key_event: &KeyEvent) -> String {
        let char;
        let key_code = match key_event.code {
            KeyCode::Backspace => "Backspace",
            KeyCode::Enter => "Enter",
            KeyCode::Left => "Left",
            KeyCode::Right => "Right",
            KeyCode::Up => "Up",
            KeyCode::Down => "Down",
            KeyCode::Home => "Home",
            KeyCode::End => "End",
            KeyCode::PageUp => "PageUp",
            KeyCode::PageDown => "PageDown",
            KeyCode::Tab => "Tab",
            KeyCode::BackTab => "Backtab",
            KeyCode::Delete => "Delete",
            KeyCode::Insert => "Insert",
            KeyCode::F(c) => {
                char = format!("F({c})");
                &char
            }
            KeyCode::Char(' ') => "Space",
            KeyCode::Char(c) => {
                char = c.to_string();
                &char
            }
            KeyCode::Esc => "Esc",
            KeyCode::Null => "",
            KeyCode::CapsLock => "",
            KeyCode::Menu => "",
            KeyCode::ScrollLock => "",
            KeyCode::Media(_) => "",
            KeyCode::NumLock => "",
            KeyCode::PrintScreen => "",
            KeyCode::Pause => "",
            KeyCode::KeypadBegin => "",
            KeyCode::Modifier(_) => "",
        };

        let mut modifiers = Vec::with_capacity(3);

        if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
            modifiers.push("Ctrl");
        }

        if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
            modifiers.push("Shift");
        }

        if key_event.modifiers.intersects(KeyModifiers::ALT) {
            modifiers.push("Alt");
        }

        let mut key = modifiers.join("-");

        if !key.is_empty() {
            key.push('-');
        }
        key.push_str(key_code);

        key
    }

    pub fn parse_key_sequence(raw: &str) -> Result<Vec<KeyEvent>, String> {
        if raw.chars().filter(|c| *c == '>').count() != raw.chars().filter(|c| *c == '<').count() {
            return Err(format!("Unable to parse `{}`", raw));
        }
        let raw = if !raw.contains("><") {
            let raw = raw.strip_prefix('<').unwrap_or(raw);
            let raw = raw.strip_prefix('>').unwrap_or(raw);
            raw
        } else {
            raw
        };
        let sequences = raw
            .split("><")
            .map(|seq| {
                if let Some(s) = seq.strip_prefix('<') {
                    s
                } else if let Some(s) = seq.strip_suffix('>') {
                    s
                } else {
                    seq
                }
            })
            .collect::<Vec<_>>();

        sequences.into_iter().map(parse_key_event).collect()
    }
}
