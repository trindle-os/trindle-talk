//! Talk: the voice plugin, which listens when the ring icon is pressed and types or reads back what it heard.
mod engine;
mod plugin;
mod settings;
mod spelling;
mod text;

use std::path::PathBuf;

fn home() -> Result<PathBuf, String> {
    trindle_common::files::home().ok_or_else(|| "HOME is not set".to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let done = match args.first().map(String::as_str) {
        Some("ring") => home().and_then(|h| plugin::ring(&h)),
        Some("say") => home().and_then(|h| engine::speak(plugin::chosen_voice(&h).as_deref(), &args[1..].join(" "))),
        _ => Err("usage: talk ring | talk say WORDS".into()),
    };
    if let Err(why) = done {
        eprintln!("talk: {why}");
        std::process::exit(1);
    }
}
