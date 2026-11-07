//! Whisper's word pieces: reads its tokenizer file and turns numbers into words.
use std::collections::HashMap;
use std::path::Path;

pub struct Tokens {
    pieces: Vec<String>,

    special: HashMap<String, u32>,
    bytes: HashMap<char, u8>,
}

fn byte_chars() -> HashMap<char, u8> {
    let mut plain: Vec<u32> = (b'!' as u32..=b'~' as u32).collect();
    plain.extend(0xA1..=0xAC);
    plain.extend(0xAE..=0xFF);
    let mut table = HashMap::new();
    let mut extra = 0;
    for b in 0u32..256 {
        let c = if plain.contains(&b) {
            b
        } else {
            extra += 1;
            255 + extra
        };
        table.insert(char::from_u32(c).unwrap(), b as u8);
    }
    table
}

impl Tokens {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let json: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let vocab = json["model"]["vocab"].as_object().ok_or("the word list is missing")?;
        let mut pieces: Vec<String> = Vec::new();
        let mut put = |id: u64, piece: &str| {
            let id = id as usize;
            if pieces.len() <= id {
                pieces.resize(id + 1, String::new());
            }
            pieces[id] = piece.to_string();
        };
        for (piece, id) in vocab {
            put(id.as_u64().ok_or("a bad word number")?, piece);
        }
        let mut special = HashMap::new();
        for added in json["added_tokens"].as_array().into_iter().flatten() {
            let (Some(id), Some(piece)) = (added["id"].as_u64(), added["content"].as_str()) else {
                continue;
            };
            put(id, piece);
            special.insert(piece.to_string(), id as u32);
        }
        Ok(Self { pieces, special, bytes: byte_chars() })
    }

    pub fn id(&self, special: &str) -> Result<u32, String> {
        self.special.get(special).copied().ok_or(format!("the word list has no {special}"))
    }

    pub fn piece(&self, id: u32) -> &str {
        self.pieces.get(id as usize).map_or("", String::as_str)
    }

    pub fn text(&self, ids: &[u32]) -> String {
        let mut raw = Vec::new();
        for &id in ids {
            let piece = self.piece(id);
            if self.special.contains_key(piece) {
                continue;
            }
            raw.extend(piece.chars().filter_map(|c| self.bytes.get(&c)));
        }
        String::from_utf8_lossy(&raw).trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = r#"{
        "model": {"vocab": {"Hello": 0, "Ġworld": 1, ".": 2, "Ã©": 3, "caf": 4}},
        "added_tokens": [{"id": 5, "content": "<|endoftext|>"}, {"id": 6, "content": "<|en|>"}]
    }"#;

    #[test]
    fn numbers_become_words() {
        let t = Tokens::parse(SMALL).unwrap();
        assert_eq!(t.text(&[6, 0, 1, 2, 5]), "Hello world.");
        assert_eq!(t.text(&[4, 3]), "café");
        assert_eq!(t.id("<|en|>").unwrap(), 6);
        assert!(t.id("<|fr|>").is_err());
    }

    #[test]
    fn the_byte_table_covers_every_byte_once() {
        let table = byte_chars();
        assert_eq!(table.len(), 256);
        assert_eq!(table[&'\u{0120}'], b' ');
        assert_eq!(table[&'A'], b'A');
    }
}
