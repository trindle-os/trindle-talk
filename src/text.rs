//! Tidies heard words: fillers out, British spelling, English only.
use crate::settings::Settings;

#[derive(Clone, Debug, PartialEq)]
enum Bit {

    Word(String),

    Gap(String),

    Mark(char),
}

fn wordish(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '\u{2019}'
}

fn split(text: &str) -> Vec<Bit> {
    let mut bits = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if wordish(c) {
            let mut word = String::from(c);
            while chars.peek().copied().is_some_and(wordish) {
                word.push(chars.next().unwrap());
            }

            if word.ends_with('\'') || word.ends_with('\u{2019}') {
                let last = word.pop().unwrap();
                if !word.is_empty() {
                    bits.push(Bit::Word(word));
                }
                bits.push(Bit::Mark(last));
            } else {
                bits.push(Bit::Word(word));
            }
        } else if c.is_whitespace() {
            let mut gap = String::from(c);
            while chars.peek().copied().is_some_and(char::is_whitespace) {
                gap.push(chars.next().unwrap());
            }
            bits.push(Bit::Gap(gap));
        } else {
            bits.push(Bit::Mark(c));
        }
    }
    bits
}

fn join(bits: &[Bit]) -> String {
    let mut out = String::new();
    let mut pending: Option<String> = None;
    for bit in bits {
        match bit {
            Bit::Gap(gap) => {
                if !out.is_empty() {
                    pending = Some(if gap.contains('\n') { gap.clone() } else { " ".into() });
                }
            }
            Bit::Mark(c) => {

                let closing = matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | ')' | ']' | '}' | '\u{2026}' | '\u{201d}' | '\u{2019}' | '%');
                if let Some(gap) = pending.take()
                    && (!closing || gap.contains('\n')) {
                        out.push_str(&gap);
                    }
                out.push(*c);
            }
            Bit::Word(word) => {
                if let Some(gap) = pending.take() {
                    out.push_str(&gap);
                }
                out.push_str(word);
            }
        }
    }
    out
}

fn is_filler(word: &str) -> bool {
    let word = word.to_lowercase();
    let run = |head: &str, tail: char, least: usize| -> bool {
        word.strip_prefix(head).is_some_and(|rest| {
            rest.len() >= least && rest.chars().all(|c| c == tail)
        })
    };
    word == "er" || word == "mm"
        || run("u", 'm', 1)
        || run("u", 'h', 1)
        || run("er", 'm', 1)
        || run("a", 'h', 1)
        || run("hm", 'm', 1)
        || run("mh", 'm', 1)
}

fn ends_sentence(c: char) -> bool {
    matches!(c, '.' | '!' | '?')
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) if !word.split_whitespace().next().is_some_and(|w| w.chars().skip(1).any(char::is_uppercase)) => first.to_uppercase().collect::<String>() + chars.as_str(),
        Some(_) => word.to_string(),
        None => String::new(),
    }
}

fn follows_number(out: &[Bit]) -> bool {
    out.iter()
        .rev()
        .find_map(|b| match b {
            Bit::Word(w) => Some(w.chars().all(|c| c.is_ascii_digit())),
            Bit::Mark(_) => Some(false),
            Bit::Gap(_) => None,
        })
        .unwrap_or(false)
}

fn skip_filler(bits: &[Bit], i: usize, out: &mut Vec<Bit>, at_start: bool, capitalise_next: &mut bool) -> usize {
    while matches!(out.last(), Some(Bit::Gap(_))) {
        out.pop();
    }
    let mut j = i + 1;
    while matches!(bits.get(j), Some(Bit::Gap(_))) {
        j += 1;
    }
    let comma_after = matches!(bits.get(j), Some(Bit::Mark(',')));
    if at_start {
        if comma_after {
            j += 1;
        }
        *capitalise_next = true;
    } else if comma_after && matches!(out.last(), Some(Bit::Mark(','))) {
        j += 1;
    }
    while matches!(bits.get(j), Some(Bit::Gap(_))) {
        j += 1;
    }
    if !out.is_empty() && matches!(bits.get(j), Some(Bit::Word(_))) {
        out.push(Bit::Gap(" ".into()));
    }
    j
}

fn tidy_marks(out: Vec<Bit>) -> Vec<Bit> {
    let mut tidy: Vec<Bit> = Vec::new();
    for bit in out {
        if let Bit::Mark(',') = bit {
            let before = tidy.iter().rev().find(|b| !matches!(b, Bit::Gap(_)));
            match before {
                None => continue,
                Some(Bit::Mark(',' | '.' | '!' | '?' | ';' | ':')) => continue,
                _ => {}
            }
        }
        if let Bit::Mark(c) = bit {
            if matches!(c, '.' | '!' | '?' | ';' | ':')
                && matches!(
                    tidy.iter().rev().find(|b| !matches!(b, Bit::Gap(_))),
                    Some(Bit::Mark(','))
                )
            {
                while !matches!(tidy.last(), Some(Bit::Mark(','))) {
                    tidy.pop();
                }
                tidy.pop();
            }
            tidy.push(Bit::Mark(c));
            continue;
        }
        tidy.push(bit);
    }
    while matches!(tidy.last(), Some(Bit::Gap(_)))
        || matches!(tidy.last(), Some(Bit::Mark(c)) if matches!(c, ',' | ';' | ':'))
    {
        tidy.pop();
    }
    tidy
}

pub fn remove_fillers(text: &str) -> String {
    let bits = split(text);
    let mut out: Vec<Bit> = Vec::new();
    let mut at_start = true;
    let mut capitalise_next = false;
    let mut i = 0;
    while i < bits.len() {
        match &bits[i] {
            Bit::Word(word) => {
                if is_filler(word) && !follows_number(&out) {
                    i = skip_filler(&bits, i, &mut out, at_start, &mut capitalise_next);
                    continue;
                }
                out.push(Bit::Word(if capitalise_next { capitalise(word) } else { word.clone() }));
                capitalise_next = false;
                at_start = false;
            }
            Bit::Mark(c) => {
                out.push(Bit::Mark(*c));
                if ends_sentence(*c) {
                    at_start = true;
                }
            }
            Bit::Gap(gap) => out.push(Bit::Gap(gap.clone())),
        }
        i += 1;
    }
    join(&tidy_marks(out)).trim().to_string()
}

fn match_case(replacement: &str, heard: &str) -> String {
    let letters: Vec<char> = heard.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() > 1 && letters.iter().all(|c| c.is_uppercase()) {
        return replacement.to_uppercase();
    }
    if heard.chars().next().is_some_and(char::is_uppercase) {
        return capitalise(replacement);
    }
    replacement.to_string()
}

pub fn apply_british(text: &str) -> String {
    let list = crate::spelling::british();
    let mut at_start = true;
    let bits: Vec<Bit> = split(text)
        .into_iter()
        .map(|bit| match bit {
            Bit::Word(word) => {

                let name = !at_start && word.chars().next().is_some_and(char::is_uppercase) && word.chars().any(char::is_lowercase);
                at_start = false;
                let better = list.get(&word.to_lowercase()).filter(|_| !name).map(|to| match_case(to, &word));
                Bit::Word(better.unwrap_or(word))
            }
            Bit::Mark(c) => {
                at_start = matches!(c, '.' | '!' | '?');
                Bit::Mark(c)
            }
            Bit::Gap(g) => {
                at_start |= g.contains('\n');
                Bit::Gap(g)
            }
        })
        .collect();
    join(&bits)
}

pub fn clean(heard: &str, settings: &Settings) -> String {
    let mut text = heard.trim().to_string();
    if text.is_empty() {
        return text;
    }
    let english = matches!(settings.language.as_deref(), None | Some("") | Some("en"));
    if settings.fillers && english {
        text = remove_fillers(&text);
    }
    if settings.british && english {
        text = apply_british(&text);
    }
    capitalise(text.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_a_hesitation_from_a_word() {
        for filler in ["um", "ummm", "uh", "uhhh", "er", "erm", "ermm", "ah", "ahh", "hmm", "hmmm", "mm", "mhm", "Um", "ER"] {
            assert!(is_filler(filler), "{filler} is a hesitation");
        }
        for real in ["under", "us", "err", "any", "a", "u", "mom", "him", "many", "hum", "erm's"] {
            assert!(!is_filler(real), "{real} is a real word");
        }
    }

    #[test]
    fn takes_a_hesitation_off_the_front() {
        assert_eq!(remove_fillers("um, hello there"), "Hello there");
        assert_eq!(remove_fillers("er I think so"), "I think so");
    }

    #[test]
    fn takes_one_out_of_the_middle() {
        assert_eq!(remove_fillers("I think um it works"), "I think it works");
        assert_eq!(remove_fillers("well, um, that is it"), "well, that is it");
    }

    #[test]
    fn takes_one_off_the_end_and_keeps_the_stop() {
        assert_eq!(remove_fillers("it is fine um."), "it is fine.");
        assert_eq!(remove_fillers("it is fine, um"), "it is fine");
    }

    #[test]
    fn a_number_being_read_out_is_left_alone() {
        assert_eq!(remove_fillers("chapter 3 er"), "chapter 3 er");
    }

    #[test]
    fn numbers_prices_and_dashes_keep_their_spacing() {
        assert_eq!(join(&split("3.5 and 1,000 at 10:30 costs $5, Tom & Jerry - wait")), "3.5 and 1,000 at 10:30 costs $5, Tom & Jerry - wait");
        assert_eq!(join(&split("trindle.org")), "trindle.org");
    }

    #[test]
    fn iphone_keeps_its_capital() {
        assert_eq!(capitalise("iPhone"), "iPhone");
        assert_eq!(capitalise("hello"), "Hello");
    }

    #[test]
    fn capitalises_the_next_sentence_too() {
        assert_eq!(remove_fillers("that is done. um, next one"), "that is done. Next one");
    }

    #[test]
    fn spells_the_british_way_and_keeps_the_shape() {
        assert_eq!(apply_british("the color of the theater"), "the colour of the theatre");
        assert_eq!(apply_british("Color COLOR color"), "Colour COLOUR colour");
        assert_eq!(apply_british("we will organize it"), "we will organise it");
    }

    #[test]
    fn names_in_the_middle_of_a_sentence_keep_their_spelling() {
        assert_eq!(apply_british("ask Gray about the gray mold"), "ask Gray about the grey mould");
    }

    #[test]
    fn everything_at_once() {
        let settings = Settings::default();
        assert_eq!(clean("um, the color of it is, er, yellow", &settings), "The colour of it is, yellow");
    }

    #[test]
    fn the_switches_turn_things_off() {
        let settings = Settings { fillers: false, british: false, ..Settings::default() };
        assert_eq!(clean("um, the color", &settings), "Um, the color");
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert_eq!(clean("   ", &Settings::default()), "");
    }
}
