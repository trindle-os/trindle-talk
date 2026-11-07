//! Talk as a Trindle plugin: what a click on its ring icon does and what it writes for the ring.
use crate::{engine, settings::Settings, text};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn notify(home: &Path, summary: &str, body: &str) {
    let _ = trindle_common::notify::notify_line(&home.join("notify.jsonl"), summary, body);
}

fn megabytes(bytes: u64) -> String {
    if bytes < 1_000_000 {
        return format!("{} KB", (bytes + 500) / 1_000);
    }
    format!("{} MB", (bytes + 500_000) / 1_000_000)
}

pub fn status(home: &Path, line: &str) {
    let _ = trindle_common::notify::status_line(&home.join("status.txt"), line);
}

fn get_ready(home: &Path, parts: &[(&str, &str)]) -> Result<bool, String> {
    let Some(engine) = engine::own_engine() else { return Ok(false) };
    let mut fetched = false;
    for (part, what) in parts {
        let mut child = Command::new(&engine)
            .args(["--fetch", part])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let errors = child.stderr.take().map(|mut err| {
            std::thread::spawn(move || {
                let mut why = String::new();
                let _ = err.read_to_string(&mut why);
                why
            })
        });
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(size) = line.strip_prefix("needs ").and_then(|n| n.trim().parse::<u64>().ok()) {
                    fetched = true;
                    status(home, &format!("Fetching {what}, {}, once only.", megabytes(size)));
                    notify(home, "Getting Talk ready",
                        &format!("Fetching {what}, {}, once only. Talk says when it is ready.", megabytes(size)));
                }
            }
        }
        let why = errors.and_then(|t| t.join().ok()).unwrap_or_default();
        if !child.wait().is_ok_and(|s| s.success()) {
            eprintln!("talk: fetching {what}: {}", why.trim());
            return Err(format!("Could not fetch {what}. Allow the network in Settings, Permissions, then press Talk again."));
        }
    }
    if fetched {
        notify(home, "Talk is ready", "Press Talk in the ring and speak.");
    }
    Ok(fetched)
}

fn microphone(runtime: Option<PathBuf>) -> Option<PathBuf> {
    Some(runtime?.join("pipewire-0")).filter(|p| p.exists())
}

fn voice(s: &Settings) -> Option<String> {
    let english = matches!(s.language.as_deref(), None | Some("en"));
    english.then(|| format!("supertonic-{}", s.supertonic_voice))
}

pub fn chosen_voice(home: &Path) -> Option<String> {
    voice(&Settings::load(home))
}

pub fn ring(home: &Path) -> Result<(), String> {
    ring_in(home, trindle_common::files::private_runtime_dir().ok())
}

fn ring_in(home: &Path, runtime: Option<PathBuf>) -> Result<(), String> {
    let settings = Settings::load(home);
    let failed = |why: String| {
        status(home, &why);
        notify(home, "Talk stopped", &why);
        Err(why)
    };
    if microphone(runtime).is_none() {
        return failed("Talk needs the microphone. Allow it in Settings, Permissions.".into());
    }
    let reading = voice(&settings).filter(|_| settings.read_back);
    let mut parts = vec![("hearing", "speech recognition")];
    if let Some(v) = &reading {
        parts.push(("supertonic", "the reading voice"));
        parts.push((v.as_str(), "the voice you chose"));
    }
    match get_ready(home, &parts) {
        Ok(true) => {
            status(home, "Ready. Press Talk in the ring and speak.");
            return Ok(());
        }
        Ok(false) => {}
        Err(why) => return failed(why),
    }
    status(home, "Listening.");
    let heard = match engine::hear(settings.language.as_deref(), || {
        notify(home, "Listening", "Speak now. Talk stops when you pause.");
    }) {
        Ok(h) => h,
        Err(why) => return failed(why),
    };
    let said = text::clean(&heard, &settings);
    if said.is_empty() {
        status(home, "Ready. Talk heard nothing last time.");
        notify(home, "Talk heard nothing", "Press Talk and speak a little louder or closer.");
        return Ok(());
    }
    status(home, "Ready. Press Talk in the ring and speak.");
    notify(home, "Talk heard", &said);
    if settings.read_back && reading.is_none() {
        notify(home, "Talk cannot read back", "Reading back works in English only.");
    }
    match reading {
        Some(v) => engine::speak(Some(&v), &said).or_else(failed),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifications_are_one_json_line_each() {
        let home = std::env::temp_dir().join(format!("talk-notify-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        notify(&home, "Talk heard", "Hello \"there\"\nfriend");
        notify(&home, "Listening", "");
        let text = std::fs::read_to_string(home.join("notify.jsonl")).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["summary"], "Talk heard");
        assert_eq!(first["body"], "Hello \"there\"\nfriend");
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn a_ring_without_the_microphone_says_so_and_stops() {
        let home = std::env::temp_dir().join(format!("talk-ring-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let why = ring_in(&home, Some(home.clone())).unwrap_err();
        assert!(why.contains("microphone"));
        let notes = std::fs::read_to_string(home.join("notify.jsonl")).unwrap();
        assert!(notes.contains("Talk stopped"));
        assert!(std::fs::read_to_string(home.join("status.txt")).unwrap().contains("microphone"));
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn reads_back_in_english_with_the_chosen_voice() {
        let mut s = Settings::default();
        assert_eq!(voice(&s).as_deref(), Some("supertonic-M5"));
        s.supertonic_voice = "M2".into();
        assert_eq!(voice(&s).as_deref(), Some("supertonic-M2"));
        s.language = Some("en".into());
        assert_eq!(voice(&s).as_deref(), Some("supertonic-M2"));
        s.language = Some("fr".into());
        assert_eq!(voice(&s), None);
        assert_eq!(megabytes(969_477_513), "969 MB");
        assert_eq!(megabytes(420_510), "421 KB");
    }

    #[test]
    fn say_uses_the_voice_chosen_in_settings() {
        let home = std::env::temp_dir().join(format!("talk-say-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("settings.ini"), "[Settings]\nvoice_supertonic=Female 1\n").unwrap();
        assert_eq!(chosen_voice(&home).as_deref(), Some("supertonic-F1"));
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn the_status_line_is_one_plain_line() {
        let home = std::env::temp_dir().join(format!("talk-status-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        status(&home, "Ready. Press Talk in the ring and speak.");
        assert_eq!(std::fs::read_to_string(home.join("status.txt")).unwrap(), "Ready. Press Talk in the ring and speak.\n");
        std::fs::remove_dir_all(home).unwrap();
    }
}
