//! What can be downloaded for voice, pinned with checksums.
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

macro_rules! pinned {
    ($name:ident, $repo:literal, $rev:literal) => {
        macro_rules! $name {
            ($path:literal) => {
                concat!("https://huggingface.co/", $repo, "/resolve/", $rev, "/", $path)
            };
        }
    };
}

pinned!(whisper, "openai/whisper-base", "e37978b90ca9030d5170a5c07aadb050351a65bb");
pinned!(supertonic, "Supertone/supertonic-2", "75e6727618a02f323c720cba9478152d4bc16ca4");

pub struct File {
    pub name: &'static str,

    pub part: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    pub url: &'static str,
}

pub const FILES: &[File] = &[
    File {
        name: "whisper-base-config.json",
        part: "hearing",
        size: 1983,
        sha256: "a153c53883a6799b6f056b4a8d1a515c9926d03994682ba88a7616618d7da0c1",
        url: whisper!("config.json"),
    },
    File {
        name: "whisper-base-tokenizer.json",
        part: "hearing",
        size: 2480466,
        sha256: "27fc476bfe7f17299480be2273fc0608e4d5a99aba2ab5dec5374b4482d1a566",
        url: whisper!("tokenizer.json"),
    },
    File {
        name: "whisper-base-model.safetensors",
        part: "hearing",
        size: 290403936,
        sha256: "07cadb9f25677c8d50df603e66a98fbd842cce45047139baeb16e6219a1e807b",
        url: whisper!("model.safetensors"),
    },
    File {
        name: "supertonic-duration_predictor.onnx",
        part: "supertonic",
        size: 1521526,
        sha256: "6d556b3691165c364be91dc0bd894656b5949f5acd2750d8ec2f954010845011",
        url: supertonic!("onnx/duration_predictor.onnx"),
    },
    File {
        name: "supertonic-text_encoder.onnx",
        part: "supertonic",
        size: 27431318,
        sha256: "dd5f535ed629f7df86071043e15f541ce1b2ab7f1bdbce4c7892b307bca79fa3",
        url: supertonic!("onnx/text_encoder.onnx"),
    },
    File {
        name: "supertonic-vector_estimator.onnx",
        part: "supertonic",
        size: 132471364,
        sha256: "105e9d66fd8756876b210a6b4aa03fc393b1eaca3a8dadcc8d9a3bc785c86a35",
        url: supertonic!("onnx/vector_estimator.onnx"),
    },
    File {
        name: "supertonic-vocoder.onnx",
        part: "supertonic",
        size: 101405066,
        sha256: "19bd51f47a186069c752403518a40f7ea4c647455056d2511f7249691ecddf7c",
        url: supertonic!("onnx/vocoder.onnx"),
    },
    File {
        name: "supertonic-unicode_indexer.json",
        part: "supertonic",
        size: 262196,
        sha256: "b7662a73a0703f43b97c0f2e089f8e8325e26f5d841aca393b5a54c509c92df1",
        url: supertonic!("onnx/unicode_indexer.json"),
    },
    File {
        name: "supertonic-voice-M1.json",
        part: "supertonic-M1",
        size: 420510,
        sha256: "a04c823cbda6dd1c7de131ec68fea83bbb70d7f29d61623304eb871e3b83b5a1",
        url: supertonic!("voice_styles/M1.json"),
    },
    File {
        name: "supertonic-voice-M2.json",
        part: "supertonic-M2",
        size: 420285,
        sha256: "7ddd07bf873a3fd67d09ef4e8293b486beb658158b47e371166198e4c6926072",
        url: supertonic!("voice_styles/M2.json"),
    },
    File {
        name: "supertonic-voice-M3.json",
        part: "supertonic-M3",
        size: 420005,
        sha256: "e8e77a56459e4dc8cdfeb88e6f778dc9a0adf22e1184414f4b0e82a5d1edbe72",
        url: supertonic!("voice_styles/M3.json"),
    },
    File {
        name: "supertonic-voice-M4.json",
        part: "supertonic-M4",
        size: 419957,
        sha256: "95322725e4d25d9ed4e7dcccbf0f3726b0e9a2471d876b7942373218dbd30174",
        url: supertonic!("voice_styles/M4.json"),
    },
    File {
        name: "supertonic-voice-M5.json",
        part: "supertonic-M5",
        size: 419619,
        sha256: "be52f82327da63ff18481ce2dd8060c7df432e0168d748745ef3e21b92d706a5",
        url: supertonic!("voice_styles/M5.json"),
    },
    File {
        name: "supertonic-voice-F1.json",
        part: "supertonic-F1",
        size: 420050,
        sha256: "6106950ebeb8a5da29ea22075f605db659cd07dbc288a68292543d9129aa250f",
        url: supertonic!("voice_styles/F1.json"),
    },
    File {
        name: "supertonic-voice-F2.json",
        part: "supertonic-F2",
        size: 420293,
        sha256: "8b97feb16d79ac0447136796708feac5f83dbabe92a5be1168212653c38729ae",
        url: supertonic!("voice_styles/F2.json"),
    },
    File {
        name: "supertonic-voice-F3.json",
        part: "supertonic-F3",
        size: 419887,
        sha256: "7eda5bccb4e6eb7f228fa182462d5fcf982d77628234603599027f0734d70c29",
        url: supertonic!("voice_styles/F3.json"),
    },
    File {
        name: "supertonic-voice-F4.json",
        part: "supertonic-F4",
        size: 419642,
        sha256: "e056fc2bee393edc8bff761eb28f33fb461e8dad828c3b05348a010ac1b7bb79",
        url: supertonic!("voice_styles/F4.json"),
    },
    File {
        name: "supertonic-voice-F5.json",
        part: "supertonic-F5",
        size: 419709,
        sha256: "ce7645ad7e3c13cca04e0d62bf890ef9ac401988005ba8f5e9c9b59257bc6931",
        url: supertonic!("voice_styles/F5.json"),
    },
];

pub fn folder() -> PathBuf {
    trindle_common::files::data_home().unwrap_or_default().join("trindle/voice")
}

const SYSTEM_FOLDER: &str = "/usr/share/trindle-voice";

pub fn folder_for(part: &str) -> PathBuf {
    folder_in(Path::new(SYSTEM_FOLDER), part)
}

fn folder_in(system: &Path, part: &str) -> PathBuf {
    if present(system, part) { system.to_path_buf() } else { folder() }
}

pub fn speech_folder(voice: &str) -> PathBuf {
    let voice = if voice.is_empty() { "supertonic-M5" } else { voice };
    let system = Path::new(SYSTEM_FOLDER);
    if present(system, "supertonic") && present(system, voice) { system.to_path_buf() } else { folder() }
}

pub fn present(dir: &Path, part: &str) -> bool {
    FILES
        .iter()
        .filter(|f| f.part == part)
        .all(|f| std::fs::metadata(dir.join(f.name)).is_ok_and(|m| m.len() == f.size))
}

pub fn sha256(path: &Path) -> std::io::Result<String> {
    let mut hash = Sha256::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn check(dir: &Path, f: &File) -> Result<(), String> {
    let path = dir.join(f.name);
    match sha256(&path) {
        Ok(sum) if sum == f.sha256 => Ok(()),
        Ok(_) => {
            let _ = std::fs::remove_file(&path);
            Err(format!("{} was damaged and has been removed", f.name))
        }
        Err(_) => Err("not-downloaded".into()),
    }
}

pub fn check_part(dir: &Path, part: &str) -> Result<(), String> {
    FILES.iter().filter(|f| f.part == part).try_for_each(|f| check(dir, f))
}

pub fn missing(dir: &Path, part: &str) -> u64 {
    FILES
        .iter()
        .filter(|f| f.part == part && !std::fs::metadata(dir.join(f.name)).is_ok_and(|m| m.len() == f.size))
        .map(|f| f.size)
        .sum()
}

pub fn fetch(dir: &Path, part: &str, mut each: impl FnMut(&File)) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for f in FILES.iter().filter(|f| f.part == part) {
        let path = dir.join(f.name);
        if std::fs::metadata(&path).is_ok_and(|m| m.len() == f.size) {
            continue;
        }
        each(f);
        fetch_file(dir, f)?;
    }
    Ok(())
}

fn fetch_file(dir: &Path, f: &File) -> Result<(), String> {
    let path = dir.join(f.name);
    let partial = dir.join(format!("{}.part", f.name));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .timeout_global(Some(Duration::from_secs(3600)))
        .build()
        .into();
    let reply = agent.get(f.url).call().map_err(|e| format!("{} could not be fetched: {e}", f.name))?;
    let mut body = reply.into_body().into_with_config().limit(f.size + 1).reader();
    let mut out = std::fs::File::create(&partial).map_err(|e| e.to_string())?;
    std::io::copy(&mut body, &mut out).map_err(|e| format!("{} stopped part way: {e}", f.name))?;
    drop(out);
    if sha256(&partial).map_err(|e| e.to_string())? != f.sha256 {
        let _ = std::fs::remove_file(&partial);
        return Err(format!("{} did not match its checksum and was removed", f.name));
    }
    std::fs::rename(&partial, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list() -> String {
    FILES
        .iter()
        .map(|f| format!("{}\t{}\t{}\t{}\t{}\n", f.part, f.name, f.size, f.sha256, f.url))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_complete_system_folder_is_read_before_the_persons_own() {
        let dir = std::env::temp_dir().join(format!("talk-system-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(folder_in(&dir, "hearing"), folder());
        for f in FILES.iter().filter(|f| f.part == "hearing") {
            std::fs::write(dir.join(f.name), vec![0u8; f.size as usize]).unwrap();
        }
        assert_eq!(folder_in(&dir, "hearing"), dir);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn every_pin_is_complete() {
        for f in FILES {
            assert_eq!(f.sha256.len(), 64, "{}", f.name);
            assert!(f.url.starts_with("https://huggingface.co/"), "{}", f.name);
            assert!(f.size > 0, "{}", f.name);
        }
    }

    fn serve_once(body: &'static [u8]) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/x", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = s.write_all(body);
        });
        url
    }

    #[test]
    fn a_fetched_file_is_kept_only_when_its_checksum_matches() {
        let dir = std::env::temp_dir().join(format!("voice-fetch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let url: &'static str = Box::leak(serve_once(b"abc").into_boxed_str());
        let bad = File { name: "bad", part: "t", size: 3, sha256: "0000000000000000000000000000000000000000000000000000000000000000", url };
        assert!(fetch_file(&dir, &bad).unwrap_err().contains("checksum"));
        assert!(!dir.join("bad").exists() && !dir.join("bad.part").exists());
        let url: &'static str = Box::leak(serve_once(b"abc").into_boxed_str());
        let good = File { name: "good", sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", url, ..bad };
        fetch_file(&dir, &good).unwrap();
        assert_eq!(std::fs::read(dir.join("good")).unwrap(), b"abc");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_damaged_file_is_thrown_away() {
        let dir = std::env::temp_dir().join(format!("voice-dl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = File { name: "x", part: "t", size: 3, sha256: "0000000000000000000000000000000000000000000000000000000000000000", url: "" };
        std::fs::write(dir.join("x"), b"abc").unwrap();
        assert!(check(&dir, &f).is_err());
        assert!(!dir.join("x").exists());
        let good = File { sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", ..f };
        std::fs::write(dir.join("x"), b"abc").unwrap();
        assert!(check(&dir, &good).is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
