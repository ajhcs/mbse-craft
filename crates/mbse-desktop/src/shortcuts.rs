//! App-owned portable shortcut overrides. Pure validation; no IO or model access.
use crate::command_catalog::{self as catalog, Id};
use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use std::collections::BTreeMap;

pub type Overrides = BTreeMap<String, String>;
#[derive(Clone)]
pub struct Bindings(BTreeMap<Id, Option<KeyboardShortcut>>);
impl Default for Bindings {
    fn default() -> Self {
        Self(
            catalog::ALL
                .iter()
                .map(|s| (s.id, Some(s.shortcut)))
                .collect(),
        )
    }
}
impl Bindings {
    pub fn get(&self, id: Id) -> Option<KeyboardShortcut> {
        self.0[&id]
    }
    pub fn resolve(overrides: &Overrides) -> Result<Self, Vec<String>> {
        if overrides.len() > 128 {
            return Err(vec![
                "At most 128 shortcut entries are supported; stored data retained.".into(),
            ]);
        }
        let mut result = Self::default();
        let mut errors = Vec::new();
        for (id, text) in overrides {
            if id.len() > 128 || text.len() > 96 {
                errors.push("Shortcut ID/value exceeds the supported byte limit.".into());
                continue;
            }
            // A future application's key stays inert and is preserved on edits.
            let Some(spec) = catalog::ALL.iter().find(|s| s.key == id) else {
                continue;
            };
            // Explicit default markers replace prior keys in the shared merge-
            // preserving store; omission alone would retain the old override.
            if text.trim().eq_ignore_ascii_case("Default") {
                result.0.insert(spec.id, Some(spec.shortcut));
                continue;
            }
            match parse(text) {
                Ok(chord) => {
                    if let Some(chord) = chord {
                        if reserved(spec.id, chord) {
                            errors.push(format!("{}: binding is reserved for text editing / native-field Undo and Redo.",spec.label));
                            continue;
                        }
                    }
                    result.0.insert(spec.id, chord);
                }
                Err(e) => errors.push(format!("{}: {e}", spec.label)),
            }
        }
        for (i, a) in catalog::ALL.iter().enumerate() {
            for b in &catalog::ALL[i + 1..] {
                if result.get(a.id).is_some() && result.get(a.id) == result.get(b.id) {
                    errors.push(format!("Conflict: {} and {} share the same effective binding (including defaults).",a.label,b.label));
                }
            }
        }
        if errors.is_empty() {
            Ok(result)
        } else {
            Err(errors)
        }
    }
}
/// Primary is Ctrl on Linux/Windows, Cmd on macOS. No unmodified typing keys.
pub fn parse(text: &str) -> Result<Option<KeyboardShortcut>, String> {
    if text.len() > 96 || !text.is_ascii() {
        return Err("Use a bounded portable binding such as Primary+Alt+U.".into());
    }
    let text = text.trim();
    if text.eq_ignore_ascii_case("Unbound") {
        return Ok(None);
    }
    let parts: Vec<_> = text.split('+').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return Err("Use Primary+[Alt+][Shift+]Key, or Unbound.".into());
    }
    let mut primary = false;
    let mut alt = false;
    let mut shift = false;
    for part in &parts[..parts.len() - 1] {
        match part.trim() {
            "Primary" if !primary => primary = true,
            "Alt" if !alt => alt = true,
            "Shift" if !shift => shift = true,
            _ => return Err("Modifiers must be unique Primary, Alt and Shift.".into()),
        }
    }
    if !primary {
        return Err(
            "Primary is required; plain typing/navigation keys stay in their widget.".into(),
        );
    }
    let name = parts.last().unwrap().trim();
    let supported = (name.len() == 1 && name.as_bytes()[0].is_ascii_alphanumeric())
        || matches!(
            name,
            "Comma"
                | "Period"
                | "Slash"
                | "Backslash"
                | "Minus"
                | "Equals"
                | "OpenBracket"
                | "CloseBracket"
                | "Space"
        )
        || (1..=12).any(|n| name == format!("F{n}"));
    if !supported {
        return Err("Supported keys: A-Z, 0-9, F1-F12, Comma/Period/Slash/Backslash/Minus/Equals/OpenBracket/CloseBracket/Space.".into());
    }
    let key = Key::from_name(&name.to_ascii_uppercase())
        .or_else(|| Key::from_name(name))
        .ok_or("Unknown shortcut key")?;
    Ok(Some(KeyboardShortcut::new(
        Modifiers {
            alt,
            shift,
            ..Modifiers::COMMAND
        },
        key,
    )))
}
fn reserved(id: Id, chord: KeyboardShortcut) -> bool {
    if chord.modifiers.alt {
        return false;
    }
    if chord.logical_key == Key::Z {
        return if chord.modifiers.shift {
            id != Id::Redo
        } else {
            id != Id::Undo
        };
    }
    if !chord.modifiers.shift
        && matches!(
            chord.logical_key,
            Key::A | Key::C | Key::V | Key::X | Key::Y
        )
    {
        return true;
    }
    chord.modifiers.shift && chord.logical_key == Key::V
}
pub fn scope(id: Id) -> &'static str {
    if id.is_history() {
        "model; fixed native-field Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z"
    } else if id.is_layout_history() {
        "canvas; inactive while editing text or an auxiliary window"
    } else {
        "workspace; existing command prerequisites apply"
    }
}
pub struct Editor {
    pub active: Bindings,
    pub draft: Overrides,
    pub load_errors: Vec<String>,
    pub open: bool,
    pub filter: String,
    pub note: String,
}
impl Editor {
    pub fn new(overrides: &Overrides) -> Self {
        let (active, load_errors) = match Bindings::resolve(overrides) {
            Ok(v) => (v, Vec::new()),
            Err(errors) => (Bindings::default(), errors),
        };
        Self {
            active,
            draft: overrides.clone(),
            load_errors,
            open: false,
            filter: String::new(),
            note: String::new(),
        }
    }
    pub fn reset_known(&mut self) {
        for s in catalog::ALL {
            if self.draft.contains_key(s.key) {
                self.draft.insert(s.key.into(), "Default".into());
            }
        }
    }
    pub fn unknown(&self) -> Vec<&str> {
        self.draft
            .keys()
            .filter(|k| !catalog::ALL.iter().any(|s| s.key == k.as_str()))
            .map(String::as_str)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_grammar_normalizes_modifiers_and_rejects_typing_or_malformed_keys() {
        assert_eq!(parse("Primary+Alt+Shift+u"), parse("Shift+Primary+Alt+U"));
        assert!(parse("Unbound").unwrap().is_none());
        for text in [
            "U",
            "Alt+U",
            "Primary+Primary+U",
            "Primary+Alt+",
            "Primary+Escape",
            "Primary+Tab",
            "Primary+F13",
            "Ctrl+U",
            "Primary+é",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
        assert!(parse(&"X".repeat(97)).is_err());
    }
    #[test]
    fn conflicts_include_defaults_and_atomic_multi_command_swaps_are_valid() {
        let mut map = Overrides::new();
        map.insert("edit.search".into(), "Primary+S".into());
        let errors = Bindings::resolve(&map).err().unwrap();
        assert!(errors
            .iter()
            .any(|e| e.contains("Conflict") && e.contains("Save") && e.contains("Search")));
        map.insert("file.save".into(), "Primary+F".into());
        let checked = Bindings::resolve(&map).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(checked.get(Id::Save), parse("Primary+F").unwrap());
        assert_eq!(checked.get(Id::Search), parse("Primary+S").unwrap());
    }
    #[test]
    fn unbinding_model_history_cannot_give_away_reserved_text_editing_keys() {
        let mut map = Overrides::new();
        map.insert("edit.undo".into(), "Unbound".into());
        assert_eq!(
            Bindings::resolve(&map)
                .unwrap_or_else(|e| panic!("{e:?}"))
                .get(Id::Undo),
            None
        );
        for text in [
            "Primary+Z",
            "Primary+Shift+Z",
            "Primary+A",
            "Primary+C",
            "Primary+X",
            "Primary+V",
            "Primary+Y",
            "Primary+Shift+V",
        ] {
            map.insert("edit.search".into(), text.into());
            assert!(
                Bindings::resolve(&map)
                    .err()
                    .unwrap()
                    .iter()
                    .any(|e| e.contains("reserved")),
                "{text}"
            );
        }
    }
    #[test]
    fn invalid_loaded_overrides_fall_back_without_rewriting_draft_and_unknowns_stay_inert() {
        let mut map = Overrides::new();
        map.insert("edit.search".into(), "Primary+S".into());
        map.insert("future.command".into(), "future syntax retained".into());
        let mut editor = Editor::new(&map);
        assert!(!editor.load_errors.is_empty());
        assert_eq!(editor.draft, map);
        assert_eq!(
            editor.active.get(Id::Search),
            Some(Id::Search.spec().shortcut)
        );
        editor.reset_known();
        assert_eq!(editor.draft.len(), 2);
        assert_eq!(editor.draft["edit.search"], "Default");
        assert_eq!(editor.unknown(), vec!["future.command"]);
        assert!(Bindings::resolve(&editor.draft).is_ok());
    }
    #[test]
    fn count_and_byte_limits_bound_configuration_and_preserve_caller_data() {
        let map: Overrides = (0..129)
            .map(|n| (format!("future.{n}"), "Unbound".into()))
            .collect();
        assert!(Bindings::resolve(&map).is_err());
        assert_eq!(map.len(), 129);
        let mut map = Overrides::new();
        map.insert("edit.undo".into(), "X".repeat(97));
        assert!(Bindings::resolve(&map).is_err());
        assert_eq!(map["edit.undo"].len(), 97);
    }
}
