//! Hearing: turns recorded sound into words with Whisper base, on this computer.
use crate::{downloads, mel, tokens::Tokens, whisper::Whisper};
use candle_core::{D, DType, Device, Tensor};
use candle_transformers::models::whisper::{self as w, Config, audio};
use candle_nn::VarBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

fn file(what: &str) -> String {
    format!("whisper-base-{what}")
}

const MOST_PIECES: usize = 224;

pub struct Hearing {
    model: Whisper,
    tokens: Tokens,
    filters: Vec<f32>,
    sot: u32,
    eot: u32,
    transcribe: u32,
    no_timestamps: u32,
    languages: Vec<u32>,

    chosen: Option<u32>,
    suppress: Tensor,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl Hearing {
    pub fn load(dir: &Path) -> Result<Self, String> {
        if !downloads::present(dir, "hearing") {
            return Err("not-downloaded".into());
        }

        for f in downloads::FILES.iter().filter(|f| f.part == "hearing") {
            downloads::check(dir, f)?;
        }

        let config: Config = serde_json::from_slice(&std::fs::read(dir.join(file("config.json"))).map_err(err)?)
            .map_err(err)?;
        let tokens = Tokens::load(&dir.join(file("tokenizer.json")))?;

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[dir.join(file("model.safetensors"))], DType::F32, &Device::Cpu) }
            .map_err(err)?;
        let model = Whisper::load(vb, config.clone()).map_err(err)?;

        let sot = tokens.id(w::SOT_TOKEN)?;
        let translate = tokens.id(w::TRANSLATE_TOKEN)?;
        let no_timestamps = tokens.id(w::NO_TIMESTAMPS_TOKEN)?;

        let languages = (sot + 1..translate).collect();
        let mask: Vec<f32> = (0..config.vocab_size as u32)
            .map(|i| {
                if config.suppress_tokens.contains(&i) || i > no_timestamps {
                    f32::NEG_INFINITY
                } else {
                    0.0
                }
            })
            .collect();
        let suppress = Tensor::new(mask.as_slice(), &Device::Cpu).map_err(err)?;
        Ok(Self {
            model,
            filters: mel::filters(),
            sot,
            eot: tokens.id(w::EOT_TOKEN)?,
            transcribe: tokens.id(w::TRANSCRIBE_TOKEN)?,
            no_timestamps,
            languages,
            chosen: None,
            suppress,
            tokens,
        })
    }

    fn language(&mut self) -> candle_core::Result<u32> {
        let scores = self.model.next(&[self.sot])?;
        let ids = Tensor::new(self.languages.as_slice(), &Device::Cpu)?;
        let best = scores.index_select(&ids, 0)?.argmax(D::Minus1)?.to_scalar::<u32>()?;
        Ok(self.languages[best as usize])
    }

    pub fn choose(&mut self, code: Option<&str>) {
        self.chosen = code.and_then(|c| self.tokens.id(&format!("<|{c}|>")).ok())
            .filter(|id| self.languages.contains(id));
    }

    fn segment(&mut self, mel: &Tensor, stop: &AtomicBool) -> candle_core::Result<Vec<u32>> {
        self.model.hear(mel)?;
        let mut new = match self.chosen {
            Some(language) => vec![self.sot, language, self.transcribe, self.no_timestamps],

            None => vec![self.language()?, self.transcribe, self.no_timestamps],
        };
        let mut pieces = Vec::new();
        while pieces.len() < MOST_PIECES && !stop.load(Ordering::Relaxed) {
            let scores = self.model.next(&new)?.broadcast_add(&self.suppress)?;
            let piece = scores.argmax(D::Minus1)?.to_scalar::<u32>()?;
            if piece == self.eot {
                break;
            }
            pieces.push(piece);
            new = vec![piece];
        }
        Ok(pieces)
    }

    pub fn words(&mut self, sound: &[f32], stop: &AtomicBool) -> Result<String, String> {
        let mut words = Vec::new();
        let mut rest = sound;
        while !rest.is_empty() && !stop.load(Ordering::Relaxed) {
            let (piece, after) = rest.split_at(cut(rest));
            let text = self.piece(piece, stop)?;
            if !text.is_empty() {
                words.push(text);
            }
            rest = after;
        }
        Ok(words.join(" "))
    }

    fn piece(&mut self, sound: &[f32], stop: &AtomicBool) -> Result<String, String> {
        let mel = audio::pcm_to_mel(&self.model.config, sound, &self.filters);
        let mels = self.model.config.num_mel_bins;
        let frames = mel.len() / mels;
        let mel = Tensor::from_vec(mel, (1, mels, frames), &Device::Cpu).map_err(err)?;
        let spoken = sound.len() / w::HOP_LENGTH;
        let mut words = Vec::new();
        let mut seek = 0;
        while seek < spoken && !stop.load(Ordering::Relaxed) {
            let size = w::N_FRAMES.min(frames - seek);
            let part = mel.narrow(2, seek, size).map_err(err)?;
            let ids = self.segment(&part, stop).map_err(err)?;
            let text = self.tokens.text(&ids);
            if !text.is_empty() {
                words.push(text);
            }
            seek += w::N_FRAMES;
        }
        Ok(words.join(" "))
    }
}

fn cut(sound: &[f32]) -> usize {
    const WHOLE: usize = 30 * 16000;
    const STEP: usize = 1600;
    if sound.len() <= WHOLE {
        return sound.len();
    }
    (WHOLE - 5 * 16000..WHOLE - STEP)
        .step_by(STEP)
        .min_by(|a, b| {
            let loud = |at: &usize| sound[*at..*at + STEP].iter().map(|x| x * x).sum::<f32>();
            loud(a).total_cmp(&loud(b))
        })
        .unwrap_or(WHOLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_recording_is_cut_in_the_quiet() {
        let mut sound = vec![0.5f32; 40 * 16000];
        sound[27 * 16000..27 * 16000 + 3200].fill(0.0);
        assert_eq!(cut(&sound), 27 * 16000);
        assert_eq!(cut(&sound[..1000]), 1000);
    }

    #[test]
    #[ignore = "needs HEAR_WAV and the downloaded model"]
    fn hears_a_recording_when_one_is_given() {
        let wav = std::env::var_os("HEAR_WAV").expect("HEAR_WAV");
        let bytes = std::fs::read(wav).unwrap();
        let sound: Vec<f32> = bytes[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b) as f32 / 32768.0)
            .collect();
        let mut hearing = Hearing::load(&downloads::folder()).unwrap();
        let begun = std::time::Instant::now();
        let words = hearing.words(&sound, &AtomicBool::new(false)).unwrap();
        eprintln!("heard in {:?}: {words}", begun.elapsed());
        if let Ok(want) = std::env::var("HEAR_WANT") {
            let flat = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
            assert_eq!(flat(&words), flat(&want));
        }
    }
}
