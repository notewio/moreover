use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug)]
enum ReplaceItem {
    WordCapture(usize),
    SuffixCapture(usize),
    String(String),
}

#[derive(Debug)]
struct Orthography {
    word: Regex,
    suffix: Regex,
    replace: Vec<ReplaceItem>,
}

pub fn apply_orthography(word: &str, suffix: &str) -> String {
    let join = word.to_string() + suffix;
    if ENGLISH_DICTIONARY.contains(&join.to_lowercase()) {
        return join;
    }
    for ortho in ORTHOGRAPHIES.iter() {
        if let (Some(word_caps), Some(suffix_caps)) =
            (ortho.word.captures(word), ortho.suffix.captures(suffix))
        {
            let mut new = String::new();
            for replace in &ortho.replace {
                new.push_str(match replace {
                    ReplaceItem::WordCapture(i) => word_caps.get(*i).unwrap().as_str(),
                    ReplaceItem::SuffixCapture(i) => suffix_caps.get(*i).unwrap().as_str(),
                    ReplaceItem::String(s) => &s,
                })
            }
            return new;
        }
    }
    join
}

macro_rules! ortho_append {
    ($word:expr, $suffix: expr, $replace: expr) => {
        Orthography {
            word: Regex::new($word).unwrap(),
            suffix: Regex::new($suffix).unwrap(),
            replace: vec![
                ReplaceItem::WordCapture(1),
                ReplaceItem::String($replace.to_string()),
            ],
        }
    };
}
macro_rules! ortho_insert {
    ($word:expr, $suffix: expr, $replace: expr) => {
        Orthography {
            word: Regex::new($word).unwrap(),
            suffix: Regex::new($suffix).unwrap(),
            replace: vec![
                ReplaceItem::WordCapture(1),
                ReplaceItem::String($replace.to_string()),
                ReplaceItem::SuffixCapture(1),
            ],
        }
    };
}
macro_rules! ortho {
    ($word:expr, $suffix: expr, $replace: expr) => {
        Orthography {
            word: Regex::new($word).unwrap(),
            suffix: Regex::new($suffix).unwrap(),
            replace: $replace,
        }
    };
    ($word:expr, $suffix: expr) => {
        Orthography {
            word: Regex::new($word).unwrap(),
            suffix: Regex::new($suffix).unwrap(),
            replace: vec![ReplaceItem::WordCapture(1), ReplaceItem::SuffixCapture(1)],
        }
    };
}

lazy_static! {
    static ref ENGLISH_DICTIONARY: HashSet<String> = HashSet::from_iter(
        include_str!("english.txt")
            .lines()
            .map(|x| x.to_string().to_lowercase())
    );
    static ref ORTHOGRAPHIES: Vec<Orthography> = vec![

        // artistic + ly = artistically
        ortho_append!(r"^(.*[aeiou]c)$", r"^ly$", r"ally"),
        // questionable +ly = questionably
        ortho_append!(r"^(.+[aeioubmnp])le$", r"^ly$", r"ly"),

        // statute + ry = statutory
        ortho_append!(r"^(.*t)e$", r"^(ry|ary)$", r"ory"),
        // confirm +tory = confirmatory (*confirmtory)
        ortho_insert!(r"^(.+)m$", r"^tor(y|ily)$", r"mator"),
        // supervise +ary = supervisory (*supervisary)
        ortho_insert!(r"^(.+)se$", r"^ar(y|ies)$", r"or"),

        // frequent + cy = frequency (tcy/tecy removal)
        ortho_append!(r"^(.*[naeiou])te?$", r"^cy$", r"cy"),

        // establish + s = establishes (sibilant pluralization)
        ortho_append!(r"^(.*(?:s|sh|x|z|zh))$", r"^s$", r"es"),
        // speech + s = speeches (soft ch pluralization).
        // NOTE: Lookarounds aren't supported, so they're just not here
        ortho_append!(r"^(.*(?:oa|ea|i|ee|oo|au|ou|l|n|[gin]ar|t)ch)$", r"^s$", r"es"),
        // cherry + s = cherries (consonant + y pluralization)
        ortho_append!(r"^(.+[bcdfghjklmnpqrstvwxz])y$", r"^s$", r"ies"),

        // die+ing = dying
        ortho_append!(r"^(.+)ie$", r"^ing$", r"ying"),
        // metallurgy + ist = metallurgist
        ortho_append!(r"^(.+[cdfghlmnpr])y$", r"^ist$", r"ist"),
        // beauty + ful = beautiful (y -> i)
        ortho_insert!(r"^(.+[bcdfghjklmnpqrstvwxz])y$", r"^([a-hj-xz].*)$", r"i"),

        // write + en = written
        ortho_append!(r"^(.+)te$", r"^en$", r"tten"),
        // Minessota +en = Minessotan (*Minessotaen)
        ortho!(r"^(.+[ae])$", r"^e(n|ns)$"),

        // ceremony +ial = ceremonial (*ceremonyial)
        ortho!(r"^(.+)y$", r"^(ial|ially)$"),

        // spaghetti +ification = spaghettification (*spaghettiification)
        ortho_insert!(r"^(.+)i$", r"^if(y|ying|ied|ies|ication|ications)$", r"if"),

        // fantastic +ical = fantastical (*fantasticcal)
        ortho!(r"^(.+)ic$", r"^(ical|ically)$"),
        // NOTE: The above regex doesn't match fantastic + al.
        // For some reason, this isn't in the Plover orthography.
        ortho!(r"^(.+ic)$", r"^(al)$"),
        // epistomology +ical = epistomological
        ortho_insert!(r"^(.+)ology$", r"^ic(al|ally)$", r"ologic"),
        // oratory +ical = oratorical (*oratoryical)
        ortho_insert!(r"^(.*)ry$", r"^ica(l|lly|lity)$", r"rica"),

        // radical +ist = radicalist (*radicallist)
        ortho_insert!(r"^(.*[l])$", r"^is(t|ts)$", r"is"),

        // complementary +ity = complementarity (*complementaryity)
        ortho_append!(r"^(.*)ry$", r"^ity$", r"rity"),
        // disproportional +ity = disproportionality (*disproportionallity)
        ortho_append!(r"^(.*)l$", r"^ity$", r"lity"),

        // perform +tive = performative (*performtive)
        ortho_insert!(r"^(.+)rm$", r"^tiv(e|ity|ities)$", r"rmativ"),
        // restore +tive = restorative
        ortho_insert!(r"^(.+)e$", r"^tiv(e|ity|ities)$", r"ativ"),

        // token +ize = tokenize (*tokennize)
        // token +ise = tokenise (*tokennise)
        ortho_insert!(r"^(.+)y$", r"^iz(e|es|ing|ed|er|ers|ation|ations|able|ability)$", r"iz"),
        ortho_insert!(r"^(.+)y$", r"^is(e|es|ing|ed|er|ers|ation|ations|able|ability)$", r"is"),
        // conditional +ize = conditionalize (*conditionallize)
        ortho_insert!(r"^(.+)al$", r"^iz(e|ed|es|ing|er|ers|ation|ations|m|ms|able|ability|abilities)$", r"aliz"),
        ortho_insert!(r"^(.+)al$", r"^is(e|ed|es|ing|er|ers|ation|ations|m|ms|able|ability|abilities)$", r"alis"),
        // spectacular +ization = spectacularization (*spectacularrization)
        ortho_insert!(r"^(.+)ar$", r"^iz(e|ed|es|ing|er|ers|ation|ations|m|ms)$", r"ariz"),
        ortho_insert!(r"^(.+)ar$", r"^is(e|ed|es|ing|er|ers|ation|ations|m|ms)$", r"aris"),

        // category +ize/+ise = categorize/categorise (*categoryize/*categoryise)
        // custom +izable/+isable = customizable/customisable (*custommizable/*custommisable)
        // fantasy +ize = fantasize (*fantasyize)
        ortho_insert!(r"^(.*[lmnty])$", r"^iz(e|es|ing|ed|er|ers|ation|ations|m|ms|able|ability|abilities)$", r"iz"),
        ortho_insert!(r"^(.*[lmnty])$", r"^is(e|es|ing|ed|er|ers|ation|ations|m|ms|able|ability|abilities)$", r"is"),

        // criminal + ology = criminology
        // criminal + ologist = criminalogist (*criminallologist)
        ortho_insert!(r"^(.+)al$", r"^olog(y|ist|ists|ical|ically)$", r"olog"),

        // similar +ish = similarish (*similarrish)
        ortho!(r"^(.+)(ar|er|or)$", r"^ish$",
            vec![ ReplaceItem::WordCapture(1), ReplaceItem::WordCapture(2), ReplaceItem::String("ish".to_string()) ]),

        // free + ed = freed
        ortho!(r"^(.+e)e$", r"^(e.+)$"),
        // narrate + ing = narrating (silent e)
        ortho!(r"^(.+[bcdfghjklmnpqrstuvwxz])e$", r"^([aeiouy].*)$"),

        // defer + ed = deferred (consonant doubling)   XXX monitor(stress not on last syllable)
        ortho!(r"^(.*(?:[bcdfghjklmnprstvwxyz]|qu)[aeiou])([bcdfgklmnprtvz])$", r"^([aeiouy].*)$",
            vec![ ReplaceItem::WordCapture(1), ReplaceItem::WordCapture(2), ReplaceItem::WordCapture(2), ReplaceItem::SuffixCapture(1) ]),

    ];
}
