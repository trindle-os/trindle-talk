//! Talk's link to the voice engine, through its two doors, voice-hear and voice-say.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Answer {
    Listening,
    Heard(String),
    Said,
    Failed(String),
    Other,
}

pub fn answer(line: &str) -> Answer {
    let line = line.trim_end_matches(['\r', '\n']);
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    match word {
        "listening" => Answer::Listening,
        "heard" => Answer::Heard(rest.split_whitespace().collect::<Vec<_>>().join(" ")),
        "said" => Answer::Said,
        "failed" => Answer::Failed(rest.trim().to_string()),
        _ => Answer::Other,
    }
}

pub fn explain(reason: &str) -> String {
    match reason {
        "not-downloaded" => "Voice is not set up. Get it from the Voice page in Settings.".into(),
        "" => "Voice stopped".into(),
        "busy" => "Voice is busy with something else. Try again in a moment.".into(),
        "voice-not-ready" => "The voice files are not all there. Open the Voice page in Settings.".into(),
        other if other.contains("pw-record") => "Talk could not use the microphone.".into(),
        other if other.contains("pw-play") => "Talk could not play sound.".into(),
        other if other.contains("model could not be loaded") => "The voice could not be loaded. Open the Voice page in Settings.".into(),
        other => {
            eprintln!("talk: voice stopped: {other}");
            "Voice stopped".into()
        }
    }
}

pub fn own_engine() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?;
    Some(me.parent()?.join("trindle-voice")).filter(|p| p.is_file())
}

fn start_engine(engine: &std::path::Path, path: &std::path::Path) {
    let started = std::process::Command::new(engine)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn();
    if started.is_err() {
        return;
    }
    for _ in 0..60 {
        if path.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn runtime() -> Result<PathBuf, String> {
    trindle_common::files::private_runtime_dir().map_err(|e| e.to_string())
}

fn door(runtime: &std::path::Path, name: &str) -> Result<UnixStream, String> {
    let path = runtime.join("trindle").join(name);
    let refused = match UnixStream::connect(&path) {
        Ok(sock) => return Ok(sock),
        Err(e) => e.kind() == std::io::ErrorKind::ConnectionRefused,
    };

    if let Some(engine) = own_engine() {
        if refused {
            let _ = std::fs::remove_file(&path);
        }
        start_engine(&engine, &path);
    }
    UnixStream::connect(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            "Voice is not set up. Talk needs the microphone and sound permissions.".to_string()
        }
        _ => "Voice is not running.".to_string(),
    })
}

pub fn hear(language: Option<&str>, listening: impl FnMut()) -> Result<String, String> {
    hear_at(&runtime()?, language, listening)
}

fn hear_at(runtime: &std::path::Path, language: Option<&str>, mut listening: impl FnMut()) -> Result<String, String> {
    let mut sock = door(runtime, "voice-hear")?;
    let ask = match language {
        Some(code) => format!("hear {code}\n"),
        None => "hear\n".to_string(),
    };
    sock.set_read_timeout(Some(std::time::Duration::from_secs(900))).map_err(|e| e.to_string())?;
    sock.write_all(ask.as_bytes()).map_err(|e| e.to_string())?;
    for line in BufReader::new(sock).lines() {
        match answer(&line.map_err(|e| e.to_string())?) {
            Answer::Listening => listening(),
            Answer::Heard(words) => return Ok(words),
            Answer::Failed(why) => return Err(explain(&why)),
            _ => {}
        }
    }
    Err(explain(""))
}

pub fn speak(voice: Option<&str>, words: &str) -> Result<(), String> {
    let runtime = runtime()?;
    let flat = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Ok(sock) = UnixStream::connect(runtime.join("trindle-voice").join("say.sock")) {
        return speak_to_service(sock, voice, &flat);
    }
    let mut sock = door(&runtime, "voice-say")?;
    let ask = match voice {
        Some(v) => format!("say-as {v} {flat}\n"),
        None => format!("say {flat}\n"),
    };
    sock.set_read_timeout(Some(std::time::Duration::from_secs(600))).map_err(|e| e.to_string())?;
    sock.write_all(ask.as_bytes()).map_err(|e| e.to_string())?;
    for line in BufReader::new(sock).lines() {
        match answer(&line.map_err(|e| e.to_string())?) {
            Answer::Said => return Ok(()),
            Answer::Failed(why) => return Err(explain(&why)),
            _ => {}
        }
    }
    Err(explain(""))
}

fn speak_to_service(mut sock: UnixStream, voice: Option<&str>, words: &str) -> Result<(), String> {
    let ask = format!("SAY id=talk priority=normal lang=en voice={}\ntext={words}\n\n", voice.unwrap_or("default"));
    sock.set_read_timeout(Some(std::time::Duration::from_secs(600))).map_err(|e| e.to_string())?;
    sock.write_all(ask.as_bytes()).map_err(|e| e.to_string())?;
    for line in BufReader::new(sock).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.starts_with("DONE") || line.starts_with("CANCELLED") {
            return Ok(());
        }
        if let Some((_, reason)) = line.strip_prefix("ERROR").and_then(|r| r.rsplit_once("reason=")) {
            return Err(explain(reason.trim()));
        }
    }
    Err(explain(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn a_failure_is_said_in_plain_words_never_as_a_code() {
        for why in ["busy", "voice-not-ready", "could not start pw-record: No such file or directory (os error 2)", "could not start pw-play: x", "the model could not be loaded", "anything else"] {
            let said = explain(why);
            assert!(!said.starts_with("Voice stopped:") && !said.contains("pw-") && !said.contains("-not-"), "{why}: {said}");
            assert!(!said.contains("os error"), "{said}");
        }
    }

    #[test]
    fn answers_are_read_and_unknown_lines_ignored() {
        assert_eq!(answer("listening\n"), Answer::Listening);
        assert_eq!(answer("heard hello there"), Answer::Heard("hello there".into()));
        assert_eq!(answer("heard"), Answer::Heard("".into()));
        assert_eq!(answer("said"), Answer::Said);
        assert_eq!(answer("failed not-downloaded"), Answer::Failed("not-downloaded".into()));
        assert_eq!(answer("something-new 1"), Answer::Other);
    }

    #[test]
    fn talks_to_a_pretend_engine() {
        let dir = std::env::temp_dir().join(format!("talk-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("trindle")).unwrap();
        let listener = UnixListener::bind(dir.join("trindle/voice-hear")).unwrap();
        let engine = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut first = String::new();
            BufReader::new(s.try_clone().unwrap()).read_line(&mut first).unwrap();
            assert_eq!(first, "hear\n");
            s.write_all(b"listening\nwhatever\nheard um hello world\n").unwrap();
        });
        let mut opened = false;
        assert_eq!(hear_at(&dir, None, || opened = true).unwrap(), "um hello world");
        assert!(opened);
        engine.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn pretend_service(reply: &'static str) -> (UnixStream, std::thread::JoinHandle<String>) {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let service = std::thread::spawn(move || {
            let mut reader = BufReader::new(theirs.try_clone().unwrap());
            let mut got = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                got.push_str(&line);
            }
            (&theirs).write_all(reply.as_bytes()).unwrap();
            got
        });
        (ours, service)
    }

    #[test]
    fn reads_aloud_through_the_system_speech_service() {
        let (sock, service) = pretend_service("STATUS ready\nSTARTED id=talk\nDONE id=talk\n");
        assert_eq!(speak_to_service(sock, Some("supertonic-F1"), "hello there"), Ok(()));
        assert_eq!(service.join().unwrap(), "SAY id=talk priority=normal lang=en voice=supertonic-F1\ntext=hello there\n");
    }

    #[test]
    fn a_service_failure_is_said_in_plain_words() {
        let (sock, service) = pretend_service("ERROR id=talk reason=voice-not-ready\n");
        let said = speak_to_service(sock, None, "hi").unwrap_err();
        assert_eq!(said, explain("voice-not-ready"));
        assert!(service.join().unwrap().contains("voice=default"));
    }
}
