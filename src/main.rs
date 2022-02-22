mod engine;
mod machine;

use directories::ProjectDirs;
use engine::Action;
use enigo::{Enigo, Key, KeyboardControllable};
use std::fs;
use toml::Value;

fn main() {
    let proj_dirs = ProjectDirs::from("", "", "moreover").unwrap();
    let file = proj_dirs.config_dir().join("moreover.toml");
    let config = fs::read_to_string(file)
        .expect("Could not read config file")
        .parse::<Value>()
        .unwrap();

    let mut engine = engine::Engine::new();
    for dict in config["dictionaries"].as_array().unwrap() {
        engine.add_dict(dict.as_str().unwrap());
    }
    let mut machine = machine::Machine::new(
        config["machine"].as_str().unwrap().to_string(),
        config["keymap"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().chars().next().unwrap_or_default())
            .collect::<Vec<char>>()
            .try_into()
            .unwrap(),
    );
    let mut enigo = Enigo::new();

    println!("Ready");

    loop {
        let stroke = machine.read().expect("Unable to read stroke");
        if stroke == 0 {
            continue;
        }

        let (del, add) = engine.process_stroke(stroke);

        for a in &del {
            match a {
                Action::Text(_) => enigo.key_click(Key::Backspace),
                _ => {}
            }
        }
        for a in &add {
            match a {
                Action::Text(s) => match s.as_str() {
                    // key_sequence doesn't seem to work for newline, i have to do this stupid
                    "\n" => enigo.key_click(Key::Return),
                    _ => enigo.key_sequence(s),
                },
                Action::KeyClick(k) => enigo.key_click(*k),
                Action::KeyUp(k) => enigo.key_up(*k),
                Action::KeyDown(k) => enigo.key_down(*k),
            }
        }
    }
}
