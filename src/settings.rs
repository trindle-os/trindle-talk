//! The settings Talk reads from its plugin folder.
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {

    pub fillers: bool,

    pub british: bool,

    pub language: Option<String>,

    pub read_back: bool,

    pub supertonic_voice: String,
}

pub const SUPERTONIC_VOICES: [(&str, &str); 10] = [
    ("Female 1", "F1"), ("Female 2", "F2"), ("Female 3", "F3"), ("Female 4", "F4"), ("Female 5", "F5"),
    ("Male 1", "M1"), ("Male 2", "M2"), ("Male 3", "M3"), ("Male 4", "M4"), ("Male 5", "M5"),
];

pub const LANGUAGES: &[(&str, &str)] = &[
    ("Any language", ""), ("English", "en"), ("Cymraeg", "cy"), ("Dansk", "da"), ("Deutsch", "de"),
    ("Español", "es"), ("Français", "fr"), ("Italiano", "it"), ("Nederlands", "nl"), ("Norsk", "no"),
    ("Polski", "pl"), ("Português", "pt"), ("Suomi", "fi"), ("Svenska", "sv"), ("Türkçe", "tr"),
    ("Русский", "ru"), ("Українська", "uk"), ("العربية", "ar"), ("हिन्दी", "hi"), ("日本語", "ja"),
];

impl Default for Settings {
    fn default() -> Self {
        Self {
            fillers: true,
            british: true,
            language: None,
            read_back: false,
            supertonic_voice: "M5".into(),
        }
    }
}

impl Settings {

    pub fn load(home: &Path) -> Self {
        let mut s = Self::default();
        let ini = trindle_common::ini::Ini::read(&home.join("settings.ini")).unwrap_or_default();
        let get = |key: &str| ini.get("Settings", key);
        for key in ["fillers", "british", "read_back"] {
            if let Some(v) = get(key) {
                let yes = v == "true";
                match key {
                    "fillers" => s.fillers = yes,
                    "british" => s.british = yes,
                    _ => s.read_back = yes,
                }
            }
        }
        if let Some(v) = get("voice_supertonic")
            && let Some(l) = SUPERTONIC_VOICES.iter().find(|l| l.0 == v) {
                s.supertonic_voice = l.1.to_string();
            }
        if let Some(v) = get("language") {
            s.language = LANGUAGES.iter().find(|l| l.0 == v && !l.1.is_empty()).map(|l| l.1.to_string());
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plugin_pages_settings_are_read() {
        let home = std::env::temp_dir().join(format!("talk-ini-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("settings.ini"),
            "[Other]\nfillers=true\n[Settings]\nfillers=false\nlanguage=Cymraeg\nread_back=true\n").unwrap();
        let s = Settings::load(&home);
        assert!(!s.fillers && s.british && s.read_back);
        assert_eq!(s.language.as_deref(), Some("cy"));
        std::fs::write(home.join("settings.ini"), "[Settings]\nlanguage=Any language\n").unwrap();
        assert_eq!(Settings::load(&home).language, None);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn the_voice_is_read() {
        let home = std::env::temp_dir().join(format!("talk-engine-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let s = Settings::load(&home);
        assert_eq!(s.supertonic_voice, "M5");
        std::fs::write(home.join("settings.ini"), "[Settings]\nvoice_supertonic=Male 3\n").unwrap();
        let s = Settings::load(&home);
        assert_eq!(s.supertonic_voice, "M3");
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn plugin_ini_offers_the_supertonic_voices() {
        let ini = include_str!("../plugin.ini");
        let part = ini.split("[Field voice_supertonic]").nth(1).unwrap();
        let line = part.lines().find(|l| l.starts_with("Choices=")).unwrap();
        let offered: Vec<&str> = line["Choices=".len()..].split(';').collect();
        let ours: Vec<&str> = SUPERTONIC_VOICES.iter().map(|v| v.0).collect();
        assert_eq!(offered, ours);
    }

    #[test]
    fn plugin_ini_offers_these_languages() {
        let ini = include_str!("../plugin.ini");
        let line = ini.lines().find(|l| l.starts_with("Choices=")).unwrap();
        let offered: Vec<&str> = line["Choices=".len()..].split(';').collect();
        let ours: Vec<&str> = LANGUAGES.iter().map(|l| l.0).collect();
        assert_eq!(offered, ours);
    }
}

#[cfg(test)]
mod spec {

    use std::collections::HashMap;

    fn parts() -> Vec<(String, HashMap<String, String>)> {
        let mut out: Vec<(String, HashMap<String, String>)> = Vec::new();
        for line in include_str!("../plugin.ini").lines().map(str::trim) {
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                out.push((name.to_string(), HashMap::new()));
            } else if let Some((k, v)) = line.split_once('=').filter(|_| !line.starts_with('#')) {
                out.last_mut().unwrap().1.insert(k.to_string(), v.to_string());
            }
        }
        out
    }

    fn plain(s: &str, most: usize) -> bool {
        s == s.trim() && !s.contains("  ") && !s.chars().any(char::is_control) && s.chars().count() <= most
    }

    #[test]
    fn plugin_ini_follows_the_format() {
        const PERMISSIONS: [&str; 13] = [
            "network", "sound", "microphone", "files", "notifications", "background", "control", "capture", "listen",
            "copied", "summon", "home-network", "usb-devices",
        ];
        let parts = parts();
        let get = |name: &str| parts.iter().find(|p| p.0 == name).map(|p| &p.1).unwrap_or_else(|| panic!("no [{name}]"));
        let plugin = get("Plugin");
        assert_eq!(plugin["Interface"], "1");
        assert!(plugin["Id"].chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') && !plugin["Id"].contains(".."));
        assert_eq!(plugin["Version"], env!("CARGO_PKG_VERSION"));
        for p in plugin["Permissions"].split(';') {
            assert!(PERMISSIONS.contains(&p), "{p} is not a permission");
        }
        let run = &get("Ring")["Run"];
        assert!(!run.starts_with('/') && !run.contains(".."));
        let fields: Vec<&str> = get("Settings")["Fields"].split(';').collect();
        for (n, name) in fields.iter().enumerate() {
            assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
            let f = get(&format!("Field {name}"));
            let kind = f["Kind"].as_str();
            assert!(["switch", "text", "choice", "note"].contains(&kind), "{name}: kind {kind}");
            match f.get("Label") {
                Some(l) => assert!(plain(l, 48), "{name}: label {l:?}"),
                None => assert!(kind == "note" && f.contains_key("TextFrom"), "{name} has no label"),
            }
            if kind == "choice" {
                let choices: Vec<&str> = f["Choices"].split(';').collect();
                assert!(choices.len() >= 2 && choices.len() <= 32 && choices.iter().all(|c| plain(c, 48)), "{name}");
                assert!(choices.contains(&f["Default"].as_str()), "{name}: default");
            }
            if kind == "switch" {
                assert!(["true", "false"].contains(&f["Default"].as_str()), "{name}: default");
            }
            if kind == "note" {
                assert!(!f.contains_key("Default"), "{name}: a note has no default");
            }
            if let Some(w) = f.get("ShowWhen") {
                let other = w.split('=').next().unwrap();
                assert!(fields[..n].contains(&other), "{name}: ShowWhen names {other}, which must come earlier");
            }
        }
        for (name, _) in &parts {
            if let Some(f) = name.strip_prefix("Field ") {
                assert!(fields.contains(&f), "[{name}] is not in Fields");
            }
        }
    }
}
