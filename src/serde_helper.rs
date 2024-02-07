pub mod keybindings {
    use std::collections::HashMap;

    use color_eyre::eyre::Result;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use derive_deref::{Deref, DerefMut};
    use serde::{de::Deserializer, Deserialize, Serialize, Serializer};

    use crate::{action::Action, app::Mode};

    #[derive(Clone, Debug, Default, Deref, DerefMut)]
    pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

    impl KeyBindings {
        #[allow(dead_code)]
        pub fn insert(&mut self, mode: Mode, key_events: &[KeyEvent], action: Action) {
            // Convert the slice of `KeyEvent`(s) to a `Vec`.
            let key_events_vec = key_events.to_vec();

            // Retrieve or create the inner `HashMap` corresponding to the mode.
            let bindings_for_mode = self.0.entry(mode).or_default();

            // Insert the `Action` into the inner `HashMap` using the key events `Vec` as
            // the key.
            bindings_for_mode.insert(key_events_vec, action);
        }

        pub fn event_to_action(&self, mode: Mode, key_events: &[KeyEvent]) -> Option<Action> {
            if key_events.is_empty() {
                None
            } else if let Some(Some(action)) = self.0.get(&mode).map(|kb| kb.get(key_events)) {
                Some(action.clone())
            } else {
                self.event_to_action(mode, &key_events[1..])
            }
        }
    }

    impl<'de> Deserialize<'de> for KeyBindings {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let parsed_map = HashMap::<Mode, HashMap<String, Action>>::deserialize(deserializer)?;

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
            let mut serialized_map: HashMap<Mode, HashMap<String, Action>> = HashMap::new();

            for (mode, key_event_map) in self.0.iter() {
                let mut string_event_map = HashMap::new();

                for (key_events, action) in key_event_map {
                    let key_string = key_events
                        .iter()
                        .map(|key_event| format!("<{}>", key_event_to_string(key_event)))
                        .collect::<Vec<String>>()
                        .join("");

                    string_event_map.insert(key_string, action.clone());
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
            KeyCode::Backspace => "backspace",
            KeyCode::Enter => "enter",
            KeyCode::Left => "left",
            KeyCode::Right => "right",
            KeyCode::Up => "up",
            KeyCode::Down => "down",
            KeyCode::Home => "home",
            KeyCode::End => "end",
            KeyCode::PageUp => "pageup",
            KeyCode::PageDown => "pagedown",
            KeyCode::Tab => "tab",
            KeyCode::BackTab => "backtab",
            KeyCode::Delete => "delete",
            KeyCode::Insert => "insert",
            KeyCode::F(c) => {
                char = format!("f({c})");
                &char
            }
            KeyCode::Char(' ') => "space",
            KeyCode::Char(c) => {
                char = c.to_string();
                &char
            }
            KeyCode::Esc => "esc",
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
            modifiers.push("ctrl");
        }

        if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
            modifiers.push("shift");
        }

        if key_event.modifiers.intersects(KeyModifiers::ALT) {
            modifiers.push("alt");
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
