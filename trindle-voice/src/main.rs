//! trindle-voice: speech to text and text to speech on this computer.
mod downloads;
mod hear;
mod listen;
mod mel;
mod speak;
mod tokens;
mod whisper;

use std::io::{BufRead, BufReader, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use engine::speech::play::{Player, Sink};

const IDLE: Duration = Duration::from_secs(60);
const BUSY_CHECK: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, PartialEq, Debug)]
enum Door {
    Hear,
    Say,
}

impl Door {
    fn name(self) -> &'static str {
        match self {
            Door::Hear => "voice-hear",
            Door::Say => "voice-say",
        }
    }
}

type Shared = Arc<Mutex<Option<hear::Hearing>>>;

fn handed_over() -> Vec<(Door, UnixListener)> {
    let ours = std::env::var("LISTEN_PID").ok().and_then(|p| p.parse::<u32>().ok())
        == Some(std::process::id());
    let count: i32 = std::env::var("LISTEN_FDS").ok().and_then(|n| n.parse().ok()).unwrap_or(0);
    if !ours || count <= 0 {
        return Vec::new();
    }
    let names = std::env::var("LISTEN_FDNAMES").unwrap_or_default();
    let names: Vec<&str> = names.split(':').collect();
    (0..count)
        .filter_map(|i| {
            let door = match names.get(i as usize).copied() {
                Some("voice-hear") => Door::Hear,
                Some("voice-say") => Door::Say,
                _ => return None,
            };

            let listener = unsafe { UnixListener::from_raw_fd(3 + i) };

            unsafe { fcntl(listener.as_raw_fd(), F_SETFD, FD_CLOEXEC) };
            Some((door, listener))
        })
        .collect()
}

const F_SETFD: i32 = 2;
const FD_CLOEXEC: i32 = 1;

unsafe extern "C" {
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
}

const DOORS: [Door; 2] = [Door::Hear, Door::Say];

fn door_path(door: Door) -> Result<PathBuf, String> {
    let runtime = trindle_common::files::private_runtime_dir().map_err(|e| e.to_string())?;
    Ok(runtime.join("trindle").join(door.name()))
}

fn open_doors() -> Result<Vec<(Door, UnixListener)>, String> {
    DOORS
        .into_iter()
        .map(|door| {
            let path = door_path(door)?;
            let folder = path.parent().ok_or("no folder")?;
            std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
            std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
            if path.exists() {

                match UnixStream::connect(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                        let _ = std::fs::remove_file(&path);
                    }
                    _ => return Err(format!("{}: another engine is already running", path.display())),
                }
            }
            let listener = UnixListener::bind(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
            Ok((door, listener))
        })
        .collect()
}

fn answer(out: &mut UnixStream, line: &str) {
    let _ = writeln!(out, "{line}");
}

fn watch_for_hang_up(reader: BufReader<UnixStream>, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        for line in reader.lines() {
            if line.is_err() {
                break;
            }
        }
        stop.store(true, Ordering::Relaxed);
    });
}

static LISTENING: AtomicBool = AtomicBool::new(false);

struct Microphone;

impl Drop for Microphone {
    fn drop(&mut self) {
        LISTENING.store(false, Ordering::SeqCst);
    }
}

fn hear(mut out: UnixStream, reader: BufReader<UnixStream>, hearing: Shared, code: Option<String>) {
    if LISTENING.swap(true, Ordering::SeqCst) {
        return answer(&mut out, "failed busy");
    }
    let _microphone = Microphone;
    let dir = downloads::folder_for("hearing");
    if !downloads::present(&dir, "hearing") {
        return answer(&mut out, "failed not-downloaded");
    }
    crate::speak::unload();
    let stop = Arc::new(AtomicBool::new(false));
    watch_for_hang_up(reader, stop.clone());

    let warming = {
        let hearing = hearing.clone();
        std::thread::spawn(move || {
            let mut slot = hearing.lock().unwrap_or_else(|p| p.into_inner());
            if slot.is_none() {
                *slot = Some(hear::Hearing::load(&dir)?);
            }
            if let Some(model) = slot.as_mut() {
                model.choose(code.as_deref());
            }
            Ok::<(), String>(())
        })
    };
    answer(&mut out, "listening");
    let recorded = listen::listen(&stop);
    let ready = warming.join().unwrap_or(Err("the model could not be loaded".into()));
    let sound = match recorded {
        Ok((_, listen::Why::Stopped)) => return,
        Ok((sound, _)) => sound,
        Err(why) => return answer(&mut out, &format!("failed {why}")),
    };
    if let Err(why) = ready {
        return answer(&mut out, &format!("failed {why}"));
    }
    if sound.is_empty() {
        return answer(&mut out, "heard");
    }
    let mut slot = hearing.lock().unwrap_or_else(|p| p.into_inner());
    let Some(model) = slot.as_mut() else { return };
    let words = model.words(&sound, &stop);

    *slot = None;
    match words {
        _ if stop.load(Ordering::Relaxed) => {}
        Ok(words) => answer(&mut out, format!("heard {}", words.split_whitespace().collect::<Vec<_>>().join(" ")).trim_end()),
        Err(why) => answer(&mut out, &format!("failed {why}")),
    }
}

fn say(mut out: UnixStream, reader: BufReader<UnixStream>, voice: &str, words: &str) {
    let sound = match speak::sound(&downloads::speech_folder(voice), voice, words) {
        Ok(s) => s,
        Err(why) => return answer(&mut out, &format!("failed {why}")),
    };
    let stop = Arc::new(AtomicBool::new(false));
    watch_for_hang_up(reader, stop.clone());
    let mut player = Player::new();
    match Sink::play(&mut player, &sound.0, sound.1, 100, &stop, &AtomicBool::new(false)) {
        _ if stop.load(Ordering::Relaxed) => {}
        Ok(_) => {
            player.finish(&stop);
            if !stop.load(Ordering::Relaxed) {
                answer(&mut out, "said");
            }
        }
        Err(why) => answer(&mut out, &format!("failed {why}")),
    }
}

fn what_to_say(line: &str) -> (String, String) {
    let line = line.trim();
    match line.split_once(' ') {
        Some(("say-as", rest)) => {
            let (voice, words) = rest.trim().split_once(' ').unwrap_or((rest.trim(), ""));
            (voice.to_string(), words.trim().to_string())
        }
        Some((_, words)) => (String::new(), words.trim().to_string()),
        None => (String::new(), String::new()),
    }
}

fn wav_sound(bytes: &[u8]) -> Result<Vec<f32>, String> {
    const BAD: &str = "not a 16-bit mono WAV file";
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(BAD.into());
    }
    let (mut rate, mut data) = (0u32, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let body = &bytes[at + 8..(at + 8).saturating_add(size).min(bytes.len())];
        match &bytes[at..at + 4] {
            b"fmt " if body.len() >= 16 => {
                let (tag, channels, bits) = (u16::from_le_bytes([body[0], body[1]]), u16::from_le_bytes([body[2], body[3]]), u16::from_le_bytes([body[14], body[15]]));
                if tag != 1 || channels != 1 || bits != 16 {
                    return Err(BAD.into());
                }
                rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            }
            b"data" => data = Some(body),
            _ => {}
        }
        at = (at + 8).saturating_add(size).saturating_add(size & 1);
    }
    let Some(data) = data.filter(|_| rate != 0) else { return Err(BAD.into()) };
    if !(4000..=384_000).contains(&rate) {
        return Err("the WAV file has an odd sample rate".into());
    }
    let at: Vec<f32> = data.as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b) as f32 / 32768.0).collect();
    let step = f64::from(rate) / 16000.0;
    let n = (at.len() as f64 / step) as usize;
    Ok((0..n)
        .map(|i| {
            let x = i as f64 * step;
            let (j, f) = (x as usize, (x.fract()) as f32);
            let next = at.get(j + 1).copied().unwrap_or(at[j]);
            at[j] * (1.0 - f) + next * f
        })
        .collect())
}

fn by_hand(args: &[String]) -> Option<Result<(), String>> {
    match args.first().map(String::as_str) {
        Some("--fetch") => {
            let part = args.get(1).map(String::as_str).unwrap_or("hearing");
            let needs = downloads::missing(&downloads::folder(), part);
            if needs > 0 {
                println!("needs {needs}");
            }
            Some(downloads::fetch(&downloads::folder(), part, |f| {
                println!("fetching {} {}", f.name, f.size);
            }))
        }
        Some("--wav") => {
            let file = args.get(1)?;

            let (voice, words) = match args.get(2) {
                Some(v) if v.starts_with("supertonic-") => (v.as_str(), args[3..].join(" ")),
                _ => ("", args[2..].join(" ")),
            };
            if words.trim().is_empty() {
                return Some(Err("--wav FILE [VOICE] WORDS".into()));
            }
            Some(speak::sound(&downloads::speech_folder(voice), voice, &words)
                .and_then(|(s, rate)| std::fs::write(file, speak::wav(&s, rate)).map_err(|e| e.to_string())))
        }
        Some("--hear-wav") => {
            let file = args.get(1)?;
            Some((|| {
                let sound = wav_sound(&std::fs::read(file).map_err(|e| e.to_string())?)?;
                let mut model = hear::Hearing::load(&downloads::folder_for("hearing"))?;
                model.choose(args.get(2).map(String::as_str));
                println!("{}", model.words(&sound, &AtomicBool::new(false))?);
                Ok(())
            })())
        }
        _ => None,
    }
}

fn serve(door: Door, stream: UnixStream, hearing: Shared) {
    let _ = stream.set_nonblocking(false);
    let Ok(read_half) = stream.try_clone() else { return };
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let word = line.split_whitespace().next().unwrap_or("");
        match (door, word) {
            (Door::Hear, "hear") => {
                let code = line.split_whitespace().nth(1).map(str::to_string);
                return hear(stream, reader, hearing, code);
            }
            (Door::Say, "say" | "say-as") => {
                let (voice, words) = what_to_say(&line);
                return say(stream, reader, &voice, &words);
            }
            _ => {}
        }
    }
}

fn main() {

    if std::env::args().nth(1).as_deref() == Some("--list") {
        print!("{}", downloads::list());
        return;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(done) = by_hand(&args) {
        if let Err(why) = done {
            eprintln!("trindle-voice: {why}");
            std::process::exit(1);
        }
        return;
    }
    let mut doors = handed_over();
    let opened_here = doors.is_empty();
    if opened_here {
        doors = match open_doors() {
            Ok(d) => d,
            Err(why) => {
                eprintln!("trindle-voice: {why}");
                std::process::exit(1);
            }
        };
    }
    let own_inodes: Vec<(Door, u64)> = if opened_here {
        doors.iter().filter_map(|(d, _)| Some((*d, std::fs::metadata(door_path(*d).ok()?).ok()?.ino()))).collect()
    } else {
        Vec::new()
    };
    let hearing: Shared = Arc::new(Mutex::new(None));
    let busy = Arc::new(AtomicUsize::new(0));
    let (knock, knocks) = std::sync::mpsc::channel();
    for (door, listener) in doors {
        let knock = knock.clone();
        std::thread::spawn(move || {
            while let Ok((stream, _)) = listener.accept() {
                if knock.send((door, stream)).is_err() {
                    return;
                }
            }
        });
    }
    let mut last = Instant::now();
    loop {
        let wait = if busy.load(Ordering::SeqCst) > 0 { BUSY_CHECK } else { IDLE.saturating_sub(last.elapsed()) };
        let mut knocked = false;
        if let Ok((door, stream)) = knocks.recv_timeout(wait) {
            knocked = true;
            let (hearing, busy) = (hearing.clone(), busy.clone());
            busy.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                serve(door, stream, hearing);
                busy.fetch_sub(1, Ordering::SeqCst);
            });
        }
        if knocked || busy.load(Ordering::SeqCst) > 0 {
            last = Instant::now();
        } else if last.elapsed() > IDLE {
            if opened_here {

                for (door, inode) in &own_inodes {
                    if let Ok(path) = door_path(*door)
                        && std::fs::metadata(&path).is_ok_and(|m| m.ino() == *inode) {
                            let _ = std::fs::remove_file(path);
                        }
                }
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wav_is_brought_to_sixteen_thousand_a_second() {
        let samples = vec![1000i16; 22050];
        let sound = wav_sound(&speak::wav(&samples, 22050)).unwrap();
        assert_eq!(sound.len(), 16000);
        assert!((sound[100] - 1000.0 / 32768.0).abs() < 1e-6);
        assert!(wav_sound(b"nonsense").is_err());
    }

    #[test]
    fn a_wav_with_an_extra_chunk_is_read_and_stereo_is_refused() {
        let plain = speak::wav(&[1000i16; 160], 16000);
        let mut with_list = plain[..36].to_vec();
        with_list.extend_from_slice(b"LIST\x03\0\0\0abc\0");
        with_list.extend_from_slice(&plain[36..]);
        assert_eq!(wav_sound(&with_list).unwrap().len(), 160);
        let mut stereo = plain.clone();
        stereo[22] = 2;
        assert!(wav_sound(&stereo).is_err());
    }

    #[test]
    fn what_to_say_is_read_from_the_line() {
        assert_eq!(what_to_say("say Hello there\n"), (String::new(), "Hello there".into()));
        assert_eq!(what_to_say("say-as af_heart Hi you\n"), ("af_heart".into(), "Hi you".into()));
        assert_eq!(what_to_say("say\n"), (String::new(), String::new()));
    }
}
