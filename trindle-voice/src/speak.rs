//! Speaking with the Supertonic 2 voices.
use std::path::Path;

use engine::speech::reader::{NORMAL_SPEED, Reader};

static READER: Reader = Reader::new(crate::downloads::check_part);

pub fn unload() {
    READER.unload();
}

pub fn sound(folder: &Path, voice: &str, words: &str) -> Result<(Vec<i16>, u32), String> {
    let mut all = Vec::new();
    let rate = READER.render(folder, voice, words, NORMAL_SPEED, None, &mut |s| {
        all.extend(s);
        true
    })?;
    Ok((all, rate))
}

pub fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
    trindle_common::wav::wav(samples, rate).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaking_says_it_is_not_ready_without_the_voice_files() {
        let dir = std::env::temp_dir().join(format!("voice-none-{}", std::process::id()));
        assert_eq!(sound(&dir, "supertonic-M5", "Hi").unwrap_err(), "voice-not-ready");
    }

    #[test]
    fn samples_become_a_wav() {
        let samples = vec![0i16; 4000];
        let w = wav(&samples, 24000);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 44 + samples.len() * 2);
    }
}
