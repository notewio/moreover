use super::{steno_to_id, Translation};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};

lazy_static! {
    static ref NUMBERS_STROKE: Regex = Regex::new(r"^#[STPHAOfpltdz]+$").unwrap();
}

struct Node {
    translation: Option<Translation>,
    children: HashMap<u32, Node>,
}
impl Node {
    fn new(t: &str, d: usize) -> Self {
        Node {
            translation: if t.len() > 0 {
                Some(Translation::new(t.to_string(), d))
            } else {
                None
            },
            children: HashMap::new(),
        }
    }
    fn get(&self, k: u32) -> Option<&Node> {
        self.children.get(&k)
    }
}

pub trait Dictionary {
    fn get(&self, strokes: &[u32]) -> Option<Translation>;
}

pub struct TreeDict {
    root: Node,
}
impl Dictionary for TreeDict {
    fn get(&self, strokes: &[u32]) -> Option<Translation> {
        let mut parent = &self.root;
        let mut trans = None;

        for s in strokes {
            if let Some(n) = parent.get(*s) {
                if let Some(t) = &n.translation {
                    trans = Some(t);
                }
                parent = &n;
            } else {
                break;
            }
        }

        if let Some(t) = trans {
            let mut trans = t.clone();
            trans.complete = parent.children.is_empty();
            return Some(trans);
        }
        None
    }
}
impl TreeDict {
    pub fn new(path: &str) -> Self {
        let file = File::open(path).unwrap();
        let lines = io::BufReader::new(file).lines();
        let mut root = Node::new("", 0);
        let mut max_depth = 0;

        let mut last_parents = vec![];
        for l in lines {
            if let Ok(line) = l {
                let mut parts = line.trim_start().split("\t");
                let id = steno_to_id(parts.next().unwrap());
                let trans = parts.next().unwrap_or("");

                let depth = line.chars().take_while(|x| x == &'\t').count();
                last_parents.resize(depth, 0);
                let mut parent = &mut root.children;
                for p in &last_parents {
                    parent = &mut parent.get_mut(p).unwrap().children;
                }

                last_parents.push(id);
                parent.insert(id, Node::new(trans, last_parents.len()));
                if last_parents.len() > max_depth {
                    max_depth = last_parents.len();
                }
            }
        }

        Self { root: root }
    }
}

macro_rules! check_contains {
    ($k: expr, $n: expr, $s: expr, $o: expr) => {
        if $k & steno_to_id($s) > 0 {
            $n.push($o);
        }
    };
}
pub struct NumbersDict {}
impl Dictionary for NumbersDict {
    fn get(&self, strokes: &[u32]) -> Option<Translation> {
        let key = strokes[0];
        if key & steno_to_id("#") == 0 || key & !steno_to_id("#STPHAOfpltdz") > 0 {
            return None;
        }
        // This is ugly and doesn't update with steno order or keys. Too bad!
        let mut number = "{&".to_string();
        check_contains!(key, number, "S", '1');
        check_contains!(key, number, "T", '2');
        check_contains!(key, number, "P", '3');
        check_contains!(key, number, "H", '4');
        check_contains!(key, number, "A", '5');
        check_contains!(key, number, "O", '0');
        check_contains!(key, number, "f", '6');
        check_contains!(key, number, "p", '7');
        check_contains!(key, number, "l", '8');
        check_contains!(key, number, "t", '9');
        if key & steno_to_id("d") > 0 {
            let n = number.pop().unwrap();
            number.push(n);
            number.push(n);
        }
        if key & steno_to_id("z") > 0 {
            number.push_str("00")
        }
        number.push('}');

        let t = Translation::new(number, 1);
        Some(t)
    }
}
impl NumbersDict {
    pub fn new() -> NumbersDict {
        NumbersDict {}
    }
}
