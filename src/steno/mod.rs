/*
    Main steno module.
    Holds structs and various utility functions and constants.
*/

pub mod engine;
mod output;

use enigo::Key;

const STENO_ORDER: &str = "^+#STKPWHRAO*eufrpblgtsdz";
const PSEUDOSTENO: [(&str, &str); 26] = [
    ("gs", "tion"),
    ("frpb", "nch"),
    ("AOeu", "ii"),
    ("AOe", "ee"),
    ("AOu", "uu"),
    ("AO", "oo"),
    ("frp", "mp"),
    ("frb", "rv"),
    ("fp", "ch"),
    ("rb", "sh"),
    ("STKPW", "Z"),
    ("TKPW", "G"),
    ("SKWR", "J"),
    ("TPH", "N"),
    ("KWR", "Y"),
    ("SR", "V"),
    ("TK", "D"),
    ("PW", "B"),
    ("HR", "L"),
    ("TP", "F"),
    ("PH", "M"),
    ("eu", "i"),
    ("pblg", "j"),
    ("pb", "n"),
    ("pl", "m"),
    ("bg", "k"),
];

#[derive(PartialEq)]
pub enum Action {
    Text(String),
    KeyDown(Key),
    KeyUp(Key),
    KeyClick(Key),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    raw: String,
    consumed: usize,
    complete: bool,
    non_undoable: bool,
}
impl Translation {
    fn new(s: String, d: usize) -> Self {
        Translation {
            raw: s,
            consumed: d,
            complete: false,
            non_undoable: false,
        }
    }
}

pub fn steno_to_id(s: &str) -> u32 {
    let mut pseudo = s.to_string();
    for (s, p) in PSEUDOSTENO {
        pseudo = pseudo.replace(p, s);
    }

    let mut result = 0u32;
    for c in pseudo.chars() {
        let index = STENO_ORDER.find(c).unwrap();
        result |= 1 << index;
    }
    result
}

fn diff<T: std::cmp::PartialEq>(a: &Vec<T>, b: &Vec<T>) -> usize {
    let mut i = 0;
    if a.len() > 0 && b.len() > 0 {
        while a[i] == b[i] {
            i += 1;
            if i >= a.len() || i >= b.len() {
                break;
            }
        }
    }
    i
}
