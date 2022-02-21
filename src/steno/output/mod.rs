mod orthography;

use super::{Action, Translation};
use enigo::Key;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref ESCAPED_BRACKETS: Regex = Regex::new(r"\\(\{|\})").unwrap();
    static ref BRACKETS: Regex = Regex::new(r"(?s)\{(.*?)\}").unwrap();
    static ref PUNCT_CAPS: Regex = Regex::new(r"^(\.|!|\?)$").unwrap();
    static ref PUNCT_SPACE: Regex = Regex::new(r"^(,|:|;)$").unwrap();
    static ref IS_COMMAND: Regex = Regex::new(r"^#.+?").unwrap();
    static ref IS_GLUE: Regex = Regex::new(r"^&.+?").unwrap();
}

macro_rules! command_keys {
    ($s: expr, $t: ident) => {
        match $s.as_str() {
            "Control_L" => Action::$t(Key::Control),
            "Shift_L" => Action::$t(Key::Shift),
            "Alt_L" => Action::$t(Key::Alt),
            "Super_L" => Action::$t(Key::Meta),

            "BackSpace" => Action::$t(Key::Backspace),
            "Escape" => Action::$t(Key::Escape),
            "Tab" => Action::$t(Key::Tab),
            "Delete" => Action::$t(Key::Delete),
            "Return" => Action::$t(Key::Return),
            "Up" => Action::$t(Key::UpArrow),
            "Down" => Action::$t(Key::DownArrow),
            "Left" => Action::$t(Key::LeftArrow),
            "Right" => Action::$t(Key::RightArrow),

            s => Action::$t(Key::Layout(s.chars().next().unwrap())),
        }
    };
}

mod format {
    pub const COMMAND: i32 = 1;
    pub const ATTACH: i32 = 1 << 1;
    pub const GLUE: i32 = 1 << 2;
    pub const CAPITALIZE: i32 = 1 << 3;
    pub const LOWERCASE: i32 = 1 << 4;
    pub const UPPERCASE: i32 = 1 << 5;

    pub const RESET_CAPS: i32 = !(CAPITALIZE | LOWERCASE | UPPERCASE);
}

pub fn translations_to_actions(translations: &mut Vec<Translation>) -> Vec<Action> {
    let mut strings = vec![];
    let mut formats = vec![0];

    for t in 0..translations.len() {
        let non_undoable = process_raw(&translations[t].raw, &mut strings, &mut formats);
        translations[t].non_undoable = non_undoable;
    }

    let mut actions = vec![];
    for i in 0..strings.len() {
        actions.append(&mut to_action(strings[i].clone(), formats[i]));
    }
    actions
}

pub fn process_raw(s: &str, strings: &mut Vec<String>, formats: &mut Vec<i32>) -> bool {
    let commands: Vec<&str> = BRACKETS
        .captures_iter(s)
        .map(|x| x.get(1).unwrap().as_str())
        .collect();
    let texts = BRACKETS.split(s);

    let mut non_undoable = true;
    for (i, s) in texts.enumerate() {
        if s.len() > 0 {
            let text = ESCAPED_BRACKETS.replace_all(s, "${1}");
            strings.push(text.to_string());
            formats.push(0);
            non_undoable = false;
        }
        if i < commands.len() {
            non_undoable &= process_command(commands[i], strings, formats);
        }
    }
    non_undoable
}

fn to_action(mut s: String, f: i32) -> Vec<Action> {
    if f & format::COMMAND > 0 {
        let mut keys = vec![];
        if s.ends_with(",") {
            s.pop();
            keys.push(command_keys!(s, KeyDown));
        } else if s.ends_with(".") {
            s.pop();
            keys.push(command_keys!(s, KeyUp));
        } else {
            keys.push(command_keys!(s, KeyClick));
        }

        keys
    } else {
        s = s.replace("\\n", "\n");
        if f & format::LOWERCASE > 0 {
            s = s.to_lowercase();
        }
        if f & format::UPPERCASE > 0 {
            s = s.to_uppercase();
        }
        let mut chars = s.chars().collect::<Vec<char>>();
        if f & format::LOWERCASE == 0 && f & format::CAPITALIZE > 0 {
            chars[0] = chars[0].to_uppercase().next().unwrap();
        }
        if f & format::ATTACH == 0 {
            chars.insert(0, ' ');
        }

        chars.iter().map(|x| Action::Text(x.to_string())).collect()
    }
}

fn process_command(s: &str, strings: &mut Vec<String>, formats: &mut Vec<i32>) -> bool {
    let mut f = formats.pop().unwrap();
    let mut next = 0;
    let mut non_undoable = true;
    if formats.len() > strings.len() {
        next = f;
        f = formats.pop().unwrap();
    }
    match s {
        // Reset formatting
        "" => {
            formats.push(0);
        }
        // Attach
        "^" | "^^" => {
            formats.push(f | format::ATTACH);
        }
        // Capitalize / Capitalize Last
        "-|" => {
            formats.push(f & format::RESET_CAPS | format::CAPITALIZE);
        }
        "*-|" => {
            if formats.len() > 0 {
                let prev = formats.pop().unwrap();
                formats.push(prev | format::CAPITALIZE);
            }
            formats.push(f);
        }
        // lowercase / lowercase last
        ">" => {
            formats.push(f & format::RESET_CAPS | format::LOWERCASE);
        }
        "*>" => {
            if formats.len() > 0 {
                let prev = formats.pop().unwrap();
                formats.push(prev | format::LOWERCASE);
                formats.push(f);
            }
        }
        // UPPERCASE / UPPERCASE LAST
        "<" => {
            formats.push(f & format::RESET_CAPS | format::UPPERCASE);
        }
        "*<" => {
            if formats.len() > 0 {
                let prev = formats.pop().unwrap();
                formats.push(prev | format::UPPERCASE);
                formats.push(f);
            }
        }
        // Carry capitalization
        "~|" => {
            formats.push(f & format::RESET_CAPS);
            formats.push(f & !format::RESET_CAPS);
        }
        _ => {
            if PUNCT_CAPS.is_match(s) {
                strings.push(s.to_string());
                formats.push(format::ATTACH);
                formats.push(format::CAPITALIZE);
                non_undoable = false;
            } else if PUNCT_SPACE.is_match(s) {
                strings.push(s.to_string());
                formats.push(format::ATTACH);
                formats.push(0);
                non_undoable = false;
            } else if IS_COMMAND.is_match(s) {
                let mut command = s.to_string();
                command.remove(0);
                strings.push(command);
                formats.push(format::COMMAND | format::ATTACH);
                formats.push(format::ATTACH);
            } else if IS_GLUE.is_match(s) {
                let mut glued = s.to_string();
                glued.remove(0);
                strings.push(glued);
                formats.push(
                    if formats.len() > 0 && formats[formats.len() - 1] & format::GLUE > 0 {
                        f | format::ATTACH | format::GLUE
                    } else {
                        f | format::GLUE
                    },
                );
                formats.push(0);
                non_undoable = false;
            } else {
                let mut text = s.to_string();
                let mut needs_orthography = false;
                if text.starts_with("^") {
                    text.remove(0);
                    f |= format::ATTACH;
                    needs_orthography = true;
                }
                if text.ends_with("^") {
                    text.pop();
                    next |= format::ATTACH;
                }
                if needs_orthography && strings.len() > 0 {
                    if formats[formats.len() - 1] & format::COMMAND == 0 {
                        let last = strings.pop().unwrap();
                        let new = orthography::apply_orthography(&last, &text);
                        strings.push(new);
                        formats.push(next);
                    } else {
                        strings.push(text);
                        formats.push(f);
                        formats.push(next);
                    }
                } else {
                    strings.push(text);
                    formats.push(f);
                    formats.push(next);
                }
                non_undoable = false;
            }
        }
    }
    non_undoable
}
