/*
    Main steno module.
    Holds structs and various utility functions and constants.
*/

mod dictionary;
mod output;

use dictionary::*;
use enigo::Key;
use output::translations_to_actions;

const BUFFER_SIZE: usize = 500;
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

pub struct Engine {
    dictionaries: Vec<Box<dyn Dictionary>>,
    strokes: Vec<u32>,
    translations: Vec<Translation>,
    suffix_folding: Vec<u32>,
}
impl Engine {
    pub fn new() -> Self {
        Engine {
            dictionaries: vec![Box::new(NumbersDict::new())],
            strokes: vec![],
            translations: vec![],
            suffix_folding: vec![
                steno_to_id("z"),
                steno_to_id("d"),
                steno_to_id("s"),
                steno_to_id("g"),
            ],
        }
    }
    pub fn add_dict(&mut self, path: &str) {
        match path.split('.').last().unwrap() {
            "txt" => {
                self.dictionaries.insert(0, Box::new(TreeDict::new(path)));
            }
            _ => {}
        }
    }

    // Take in a stroke, translate it, compare it with the previous state, and
    // return necessary deletions and additions.
    pub fn process_stroke(&mut self, stroke: u32) -> (Vec<Action>, Vec<Action>) {
        self.flush_buffer();

        let ti = self
            .translations
            .iter()
            .take_while(|x| x.complete && x.consumed > 0)
            .count()
            .saturating_sub(1); // We want that extra stroke in case we need to undo
        let mut old_translations = self.translations[ti..].to_vec();
        let mut stroke_length: usize = old_translations.iter().map(|x| x.consumed).sum();

        if stroke == steno_to_id("*") {
            if self.strokes.len() == 0 {
                return (vec![], vec![]);
            }
            let num_non_undoable = self
                .translations
                .iter()
                .rev()
                .take_while(|x| x.non_undoable)
                .count()
                + 1;
            self.strokes
                .truncate(self.strokes.len().saturating_sub(num_non_undoable));
            stroke_length = stroke_length.saturating_sub(num_non_undoable);
        } else {
            self.strokes.push(stroke);
            stroke_length += 1;
        }

        let new_strokes = &self.strokes[self.strokes.len() - stroke_length..];
        let mut new_translations = self.translate_strokes(new_strokes);

        let mut old_actions = translations_to_actions(&mut old_translations);
        let mut new_actions = translations_to_actions(&mut new_translations);

        let di = diff(&old_translations, &new_translations);
        self.translations.drain(ti + di..);
        for i in di..new_translations.len() {
            self.translations.push(new_translations[i].clone());
        }

        let ai = diff(&old_actions, &new_actions);
        old_actions.drain(0..ai);
        new_actions.drain(0..ai);
        (old_actions, new_actions)
    }

    // Takes a slice of strokes, and greedily translates them.
    fn translate_strokes(&self, strokes: &[u32]) -> Vec<Translation> {
        let mut translations = vec![];

        let mut i = 0;
        while i < strokes.len() {
            let trans = self.lookup(&strokes[i..]);
            // Translation exists
            if let (Some(t), s) = trans {
                i += t.consumed;
                translations.push(t);
                if let Some(st) = s {
                    translations.push(st);
                }
            }
            // Failed translation
            else {
                translations.push(Translation::new(strokes[i].to_string(), 1));
                i += 1;
            }
        }

        translations
    }

    // Lookup in each dictionary, by priority.
    // Second item in tuple is for the folded suffix.
    fn lookup(&self, strokes: &[u32]) -> (Option<Translation>, Option<Translation>) {
        let trans = self.lookup_helper(strokes);
        if trans.is_some() {
            return (trans, None);
        }
        // Suffix folding:
        // Mask the suffix in the last stroke, and retry the lookup
        for suffix in &self.suffix_folding {
            let suffix_trans = self.lookup_helper(&[*suffix]);
            if let Some(mut st) = suffix_trans {
                st.consumed = 0;

                // NOTE: really hacky and kind of deletes the benefit of tree dict, but idk how else to do it
                // mabye include suffix folding in dict struct instead? but then passing the suffix trans... :(
                for i in (1..strokes.len() + 1).rev() {
                    let mut new_search = strokes[..i].to_vec();
                    let masked = new_search.pop().unwrap() & !suffix;
                    new_search.push(masked);
                    let trans = self.lookup_helper(&new_search);
                    if trans.is_some() {
                        return (trans, Some(st));
                    }
                }
            }
        }
        (None, None)
    }

    // Get a translation.
    fn lookup_helper(&self, strokes: &[u32]) -> Option<Translation> {
        for dict in &self.dictionaries {
            let trans = dict.get(strokes);
            if trans.is_some() {
                return trans;
            }
        }
        None
    }

    // Limit the stroke buffer size.
    fn flush_buffer(&mut self) {
        let mut n = 0;
        let mut ti = self.translations.len();
        while ti > 0 && n < BUFFER_SIZE {
            ti -= 1;
            n += self.translations[ti].consumed;
        }
        self.translations.drain(0..ti);
        self.strokes.drain(0..self.strokes.len() - n);
    }
}
