use crate::hangul::{conjugate, Ending, Pos, REGULAR_ENDINGS};
use std::collections::HashMap;
use std::sync::OnceLock;

pub const PARTICLES: &[(&str, &str, &str)] = &[
    ("은", "topic", "는"),
    ("는", "topic", "는"),
    ("이", "subject", "가"),
    ("가", "subject", "가"),
    ("을", "object", "를"),
    ("를", "object", "를"),
    ("에서", "ablative", "에서"),
    ("에게", "dative_person", "에게"),
    ("에", "dative", "에"),
    ("으로", "instrument", "로"),
    ("로", "instrument", "로"),
    ("의", "genitive", "의"),
    ("보다", "comparative", "보다"),
    ("마다", "distributive", "마다"),
    ("만큼", "quantity", "만큼"),
    ("씩", "step", "씩"),
    ("부터", "from", "부터"),
    ("까지", "to", "까지"),
    ("와", "conj", "와"),
    ("과", "conj", "와"),
];

pub fn particles_by_length() -> Vec<(String, &'static str, &'static str)> {
    let mut all: Vec<(String, &'static str, &'static str)> = PARTICLES
        .iter()
        .map(|&(form, role, canon)| (form.to_string(), role, canon))
        .collect();
    all.sort_by_key(|(form, _, _)| std::cmp::Reverse(form.chars().count()));
    all
}

pub const STRUCTURAL: &[&str] = &["마다", "부터", "까지", "씩", "모듈"];

pub const FIELDS: &[&str] = &["자료형", "길이", "명칭"];

pub const KEYWORDS: &[&str] = &[
    "만약",
    "아니고",
    "아니면",
    "동안",
    "참",
    "거짓",
    "없음",
    "묶음",
];

pub const HADA_FORMS: &[(&str, Ending)] = &[
    ("하는지", Ending::Interrogative),
    ("하거나", Ending::Alternative),
    ("하는", Ending::AdnominalPres),
    ("하면", Ending::Conditional),
    ("하고", Ending::Conjunctive),
    ("하지", Ending::Negative),
    ("한다", Ending::Final),
    ("한", Ending::AdnominalPast),
    ("해", Ending::Auxiliary),
];

pub const DOEDA_FORMS: &[(&str, Ending)] = &[
    ("되는지", Ending::Interrogative),
    ("되는", Ending::AdnominalPres),
    ("되면", Ending::Conditional),
    ("되고", Ending::Conjunctive),
    ("된다", Ending::Final),
    ("된", Ending::AdnominalPast),
];

pub const COPULA: &[(&str, Ending)] = &[
    ("이거나", Ending::Alternative),
    ("이라는", Ending::Quotative),
    ("라는", Ending::Quotative),
    ("이면", Ending::Conditional),
    ("인지", Ending::Interrogative),
    ("이고", Ending::Conjunctive),
    ("이다", Ending::Final),
    ("인", Ending::AdnominalPast),
];

pub const COMPARATIVES: &[(&str, &str, bool)] = &[
    ("이상", "작다", true),
    ("이하", "크다", true),
    ("초과", "크다", false),
    ("미만", "작다", false),
];

pub const CALL_TAILS: &[&str] = &["값", "나머지", "몫"];

pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

pub fn particle(form: &str) -> Option<(&'static str, &'static str)> {
    PARTICLES
        .iter()
        .find(|&&(f, _, _)| f == form)
        .map(|&(_, role, canon)| (role, canon))
}

pub struct Builtin {
    pub name: &'static str,
    stem: &'static str,
    pos: Pos,
    overrides: &'static [(Ending, &'static str)],
}

const fn verb(name: &'static str, stem: &'static str) -> Builtin {
    Builtin {
        name,
        stem,
        pos: Pos::Verb,
        overrides: &[],
    }
}

pub const BUILTIN_VERBS: &[Builtin] = &[
    verb("출력하다", "출력하"),
    verb("추가하다", "추가하"),
    verb("제거하다", "제거하"),
    verb("바꾸다", "바꾸"),
    verb("더하다", "더하"),
    verb("빼다", "빼"),
    verb("곱하다", "곱하"),
    verb("나누다", "나누"),
    verb("반복하다", "반복하"),
    verb("빠져나가다", "빠져나가"),
    verb("하다", "하"),
    verb("읽다", "읽"),
    verb("쓰다", "쓰"),
    verb("가져오다", "가져오"),
    verb("넘어가다", "넘어가"),
    verb("반환하다", "반환하"),
    Builtin {
        name: "잇다",
        stem: "잇",
        pos: Pos::Verb,
        overrides: &[
            (Ending::Auxiliary, "이어"),
            (Ending::AdnominalPast, "이은"),
            (Ending::Conditional, "이으면"),
        ],
    },
    Builtin {
        name: "않다",
        stem: "않",
        pos: Pos::Descriptive,
        overrides: &[
            (Ending::Final, "않다"),
            (Ending::AdnominalPast, "않은"),
            (Ending::AdnominalPres, "않는"),
            (Ending::Conditional, "않으면"),
            (Ending::Conjunctive, "않고"),
            (Ending::Interrogative, "않은지"),
            (Ending::Alternative, "않거나"),
        ],
    },
    Builtin {
        name: "아니다",
        stem: "아니",
        pos: Pos::Descriptive,
        overrides: &[
            (Ending::Final, "아니다"),
            (Ending::AdnominalPast, "아닌"),
            (Ending::AdnominalPres, "아닌"),
            (Ending::Interrogative, "아닌지"),
            (Ending::Conjunctive, "아니고"),
            (Ending::Conditional, "아니면"),
        ],
    },
    verb("열다", "열"),
    verb("닫다", "닫"),
    Builtin {
        name: "크다",
        stem: "크",
        pos: Pos::Descriptive,
        overrides: &[],
    },
    Builtin {
        name: "작다",
        stem: "작",
        pos: Pos::Descriptive,
        overrides: &[],
    },
    Builtin {
        name: "같다",
        stem: "같",
        pos: Pos::Descriptive,
        overrides: &[],
    },
];

pub type FormTable = HashMap<String, (String, Pos, Ending)>;

pub fn builtin_forms() -> &'static FormTable {
    static TABLE: OnceLock<FormTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = FormTable::new();
        for found in BUILTIN_VERBS {
            for &ending in &REGULAR_ENDINGS {
                let surface = found
                    .overrides
                    .iter()
                    .find(|&&(which, _)| which == ending)
                    .map(|&(_, form)| form.to_string())
                    .or_else(|| conjugate(found.stem, found.pos, ending));
                if let Some(surface) = surface {
                    table
                        .entry(surface)
                        .or_insert((found.name.into(), found.pos, ending));
                }
            }
        }
        table
    })
}

pub fn stem_forms<'a>(stems: impl IntoIterator<Item = &'a String>) -> FormTable {
    let mut sorted: Vec<&String> = stems.into_iter().collect();
    sorted.sort();
    let mut table = FormTable::new();
    for stem in sorted {
        let name = format!("{stem}다");
        for &ending in &REGULAR_ENDINGS {
            let regular = conjugate(stem, Pos::Verb, ending);
            let odd = crate::hangul::irregular(stem, ending);
            for surface in regular.into_iter().chain(odd) {
                table
                    .entry(surface)
                    .or_insert((name.clone(), Pos::Verb, ending));
            }
        }
    }
    table
}

pub fn copula_suffix(
    chunk: &str,
    known: &dyn Fn(&str) -> bool,
) -> Option<(&'static str, Ending)> {
    let mut fits: Vec<(&'static str, Ending)> = COPULA
        .iter()
        .filter(|&&(form, _)| {
            chunk
                .strip_suffix(form)
                .is_some_and(|head| !head.is_empty())
        })
        .copied()
        .collect();
    fits.sort_by_key(|(form, _)| form.chars().count());
    for &(form, ending) in &fits {
        if known(chunk.strip_suffix(form).unwrap()) {
            return Some((form, ending));
        }
    }
    fits.into_iter()
        .max_by_key(|(form, _)| form.chars().count())
}
