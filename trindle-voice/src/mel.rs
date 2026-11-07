//! The 80 mel filters Whisper hears through.
pub const MELS: usize = 80;
pub const BINS: usize = 201;

const RATE: f64 = 16000.0;
const N_FFT: f64 = 400.0;

const F_SP: f64 = 200.0 / 3.0;
const MIN_LOG_HZ: f64 = 1000.0;
const MIN_LOG_MEL: f64 = MIN_LOG_HZ / F_SP;

fn logstep() -> f64 {
    6.4f64.ln() / 27.0
}

fn hz_to_mel(hz: f64) -> f64 {
    if hz >= MIN_LOG_HZ {
        MIN_LOG_MEL + (hz / MIN_LOG_HZ).ln() / logstep()
    } else {
        hz / F_SP
    }
}

fn mel_to_hz(mel: f64) -> f64 {
    if mel >= MIN_LOG_MEL {
        MIN_LOG_HZ * (logstep() * (mel - MIN_LOG_MEL)).exp()
    } else {
        F_SP * mel
    }
}

pub fn filters() -> Vec<f32> {
    let fft_hz: Vec<f64> = (0..BINS).map(|i| i as f64 * RATE / N_FFT).collect();
    let top = hz_to_mel(RATE / 2.0);
    let points: Vec<f64> = (0..MELS + 2)
        .map(|i| mel_to_hz(top * i as f64 / (MELS + 1) as f64))
        .collect();
    let mut out = Vec::with_capacity(MELS * BINS);
    for m in 0..MELS {
        let (lo, mid, hi) = (points[m], points[m + 1], points[m + 2]);
        let norm = 2.0 / (hi - lo);
        for &f in &fft_hz {
            let rising = (f - lo) / (mid - lo);
            let falling = (hi - f) / (hi - mid);
            out.push((rising.min(falling).max(0.0) * norm) as f32);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_the_filters_whisper_was_trained_with() {
        let f = filters();
        assert_eq!(f.len(), MELS * BINS);
        let near = |i: usize, want: f32| assert!((f[i] - want).abs() < 1e-6, "{i}: {} vs {want}", f[i]);
        near(0, 0.0);
        near(1, 0.024863);
        near(2, 0.0);
        near(202, 0.0019908);
        near(203, 0.0228718);
        near(204, 0.0);
        let max = f.iter().cloned().fold(0.0, f32::max);
        assert!((max - 0.025880683).abs() < 1e-7, "{max}");
    }

    #[test]
    #[ignore = "needs MEL_REFERENCE, candle's filter file"]
    fn match_every_value_when_the_reference_is_present() {
        let path = std::env::var_os("MEL_REFERENCE").expect("MEL_REFERENCE");
        let bytes = std::fs::read(path).unwrap();
        let want: Vec<f32> = bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
        let got = filters();
        assert_eq!(want.len(), got.len());
        let worst = want.iter().zip(&got).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
        assert!(worst < 1e-6, "worst difference {worst}");
    }
}
