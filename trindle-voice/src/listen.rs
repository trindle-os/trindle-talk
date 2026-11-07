//! Recording what someone says, and noticing when they have finished.
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const RATE: usize = 16_000;

const LOOK: Duration = Duration::from_millis(150);

const QUIET_AFTER: Duration = Duration::from_millis(1600);

const NOTHING_AFTER: Duration = Duration::from_secs(6);

const MAX_RECORDING: Duration = Duration::from_secs(600);

const WINDOW: usize = 4800;

const FLOOR_CEILING: f64 = 600.0;
const WARMUP: Duration = Duration::from_millis(500);

const TOO_SHORT: usize = RATE / 2;

fn loudness(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| *s as f64 * *s as f64).sum();
    (sum / samples.len() as f64).sqrt()
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Why {

    Finished,

    Nothing,

    TooLong,

    Stopped,
}

struct Silence {
    started: Instant,
    floor: f64,
    spoke: bool,
    quiet_since: Option<Instant>,
}

impl Silence {
    fn new(now: Instant) -> Self {
        Self { started: now, floor: f64::MAX, spoke: false, quiet_since: None }
    }

    fn look(&mut self, level: f64, now: Instant) -> Option<Why> {
        if now.duration_since(self.started) < WARMUP {
            return None;
        }

        self.floor = self.floor.min(level.max(30.0)).min(FLOOR_CEILING);
        let speaking = level > (self.floor * 3.0).max(400.0);
        if speaking {
            self.spoke = true;
            self.quiet_since = None;
        } else if self.quiet_since.is_none() {
            self.quiet_since = Some(now);
        }
        if now.duration_since(self.started) >= MAX_RECORDING {
            return Some(Why::TooLong);
        }
        let quiet_long = self.quiet_since.is_some_and(|t| now.duration_since(t) >= QUIET_AFTER);
        if self.spoke && quiet_long {
            return Some(Why::Finished);
        }
        if !self.spoke && now.duration_since(self.started) >= NOTHING_AFTER {
            return Some(Why::Nothing);
        }
        None
    }
}

fn until_done(mut read: impl FnMut(&mut [u8]) -> std::io::Result<usize>) -> (Vec<i16>, Why) {
    let base = Instant::now();
    let mut silence = Silence::new(base);
    let mut samples: Vec<i16> = Vec::new();
    let mut odd: Option<u8> = None;
    let mut buf = [0u8; 3200];
    let mut next_look = LOOK;
    loop {
        let n = match read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let mut bytes = &buf[..n];
        if let Some(lo) = odd.take() {
            samples.push(i16::from_le_bytes([lo, bytes[0]]));
            bytes = &bytes[1..];
        }
        let (pairs, rest) = bytes.as_chunks::<2>();
        samples.extend(pairs.iter().map(|b| i16::from_le_bytes(*b)));
        odd = rest.first().copied();
        let heard = Duration::from_millis((samples.len() * 1000 / RATE) as u64);
        while heard >= next_look {
            let from = samples.len().saturating_sub(WINDOW);
            if let Some(why) = silence.look(loudness(&samples[from..]), base + next_look) {
                return (samples, why);
            }
            next_look += LOOK;
        }
    }

    let why = if silence.spoke { Why::Finished } else { Why::Nothing };
    (samples, why)
}

pub fn listen(stop: &AtomicBool) -> Result<(Vec<f32>, Why), String> {
    let mut child = Command::new("pw-record")
        .args(["--raw", "--rate", "16000", "--channels", "1", "--format", "s16", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start pw-record: {e}"))?;
    let mut out = child.stdout.take().ok_or("pw-record gave no output")?;

    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 3200];
        while let Ok(n) = out.read(&mut buf) {
            if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let mut last_data = Instant::now();
    let (samples, why) = until_done(|buf| loop {
        if stop.load(Ordering::Relaxed) || last_data.elapsed() > Duration::from_secs(3) {
            return Ok(0);
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(bytes) => {
                last_data = Instant::now();
                let n = bytes.len().min(buf.len());
                buf[..n].copy_from_slice(&bytes[..n]);
                return Ok(n);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => return Ok(0),
        }
    });
    let _ = child.kill();
    let _ = child.wait();
    if stop.load(Ordering::Relaxed) {
        return Ok((Vec::new(), Why::Stopped));
    }
    if why == Why::Nothing || samples.len() < TOO_SHORT {
        return Ok((Vec::new(), Why::Nothing));
    }
    Ok((samples.iter().map(|s| *s as f32 / 32768.0).collect(), why))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    #[test]
    fn measures_how_loud_a_moment_is() {
        assert_eq!(loudness(&[]), 0.0);
        assert_eq!(loudness(&[0, 0, 0, 0]), 0.0);
        let tone = vec![3276i16, -3276, 3276, -3276];
        assert!((loudness(&tone) - 3276.0).abs() < 1.0);
    }

    #[test]
    fn waits_for_a_pause_after_speaking() {
        let base = Instant::now();
        let mut silence = Silence::new(base);
        assert_eq!(silence.look(40.0, at(base, 600)), None);
        assert_eq!(silence.look(3000.0, at(base, 750)), None);
        assert_eq!(silence.look(3000.0, at(base, 1000)), None);
        assert_eq!(silence.look(45.0, at(base, 1200)), None);

        assert_eq!(silence.look(45.0, at(base, 2000)), None);
        assert_eq!(silence.look(3000.0, at(base, 2200)), None);
        assert_eq!(silence.look(45.0, at(base, 2400)), None);
        assert_eq!(silence.look(45.0, at(base, 4100)), Some(Why::Finished));
    }

    #[test]
    fn the_recorders_own_starting_burst_is_not_speech() {
        let base = Instant::now();
        let mut silence = Silence::new(base);
        for (ms, level) in [(150, 32760.0), (300, 17816.0), (450, 2891.0)] {
            assert_eq!(silence.look(level, at(base, ms)), None);
        }
        assert!(!silence.spoke);
        for ms in (600..6000).step_by(150) {
            assert_eq!(silence.look(60.0, at(base, ms)), None, "at {ms}ms");
        }
        assert_eq!(silence.look(60.0, at(base, 6000)), Some(Why::Nothing));
    }

    #[test]
    fn speech_from_the_very_start_is_still_speech() {
        let base = Instant::now();
        let mut silence = Silence::new(base);
        for ms in (600..3000).step_by(150) {
            assert_eq!(silence.look(3000.0, at(base, ms)), None);
        }
        assert!(silence.spoke);
        assert_eq!(silence.look(45.0, at(base, 3150)), None);
        assert_eq!(silence.look(45.0, at(base, 4800)), Some(Why::Finished));
    }

    #[test]
    fn a_noisy_room_is_not_speech() {
        let base = Instant::now();
        let mut silence = Silence::new(base);
        for ms in (150..6000).step_by(150) {
            assert_eq!(silence.look(900.0, at(base, ms)), None, "at {ms}ms");
        }
        assert_eq!(silence.look(900.0, at(base, 6000)), Some(Why::Nothing));
    }

    #[test]
    fn stops_a_dictation_that_never_ends() {
        let base = Instant::now();
        let mut silence = Silence::new(base);
        assert_eq!(silence.look(40.0, at(base, 600)), None);
        let mut ms = 750;
        while ms < MAX_RECORDING.as_millis() as u64 {
            assert_eq!(silence.look(5000.0, at(base, ms)), None, "at {ms}ms");
            ms += 150;
        }
        assert_eq!(silence.look(5000.0, at(base, ms)), Some(Why::TooLong));
    }

    #[test]
    fn stops_a_stream_after_speech_then_quiet() {
        let mut pcm: Vec<i16> = Vec::new();
        pcm.extend(std::iter::repeat_n(20, RATE));
        pcm.extend((0..2 * RATE).map(|i| if i % 2 == 0 { 6000 } else { -6000 }));
        pcm.extend(std::iter::repeat_n(20, 5 * RATE));
        let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut at = 0;
        let (samples, why) = until_done(|buf| {
            let n = 777.min(buf.len()).min(bytes.len() - at);
            buf[..n].copy_from_slice(&bytes[at..at + n]);
            at += n;
            Ok(n)
        });
        assert_eq!(why, Why::Finished);

        let secs = samples.len() as f64 / RATE as f64;
        assert!(secs > 4.5 && secs < 5.5, "stopped at {secs}s");
        assert_eq!(samples[RATE + 1], -6000);
    }
}
