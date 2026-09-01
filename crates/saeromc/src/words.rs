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
    ("부터", "from", "부터"),
    ("까지", "to", "까지"),
];

const RANGE_TAILS: &[&str] = &["의", "에", "를", "을"];

pub fn particles_by_length() -> Vec<(String, &'static str, &'static str)> {
    let mut all: Vec<(String, &'static str, &'static str)> = PARTICLES
        .iter()
        .map(|&(form, role, canon)| (form.to_string(), role, canon))
        .collect();
    for &(form, role, canon) in PARTICLES {
        if role == "from" || role == "to" {
            all.extend(
                RANGE_TAILS
                    .iter()
                    .map(|t| (format!("{form}{t}"), role, canon)),
            );
        }
    }
    all.sort_by_key(|(form, _, _)| std::cmp::Reverse(form.chars().count()));
    all
}

pub const STRUCTURAL: &[&str] = &["마다", "부터", "까지", "간격", "모듈"];

pub const KEYWORDS: &[&str] = &[
    "만약",
    "아니고",
    "아니면",
    "동안",
    "참",
    "거짓",
    "간격",
    "끝으로",
    "오류",
];

pub const HADA_FORMS: &[(&str, &str)] = &[
    ("하는지", "interrogative"),
    ("하거나", "alternative"),
    ("하는", "adnominal_pres"),
    ("하면", "conditional"),
    ("하고", "conjunctive"),
    ("하지", "negative"),
    ("한다", "final"),
    ("한", "adnominal_past"),
    ("해", "auxiliary"),
];

pub const DOEDA_FORMS: &[(&str, &str)] = &[
    ("되는지", "interrogative"),
    ("되는", "adnominal_pres"),
    ("되면", "conditional"),
    ("되고", "conjunctive"),
    ("된다", "final"),
    ("된", "adnominal_past"),
];

/// `다`, `면` 넣지 말 것
pub const COPULA: &[(&str, &str)] = &[
    ("이거나", "alternative"),
    ("이라는", "quotative"),
    ("라는", "quotative"),
    ("이면", "conditional"),
    ("인지", "interrogative"),
    ("이고", "conjunctive"),
    ("이다", "final"),
    ("인", "adnominal_past"),
];

/// `초` + `과` 방지
pub const COMPARATIVES: &[(&str, &str, bool)] = &[
    ("이상", "작다", true),
    ("이하", "크다", true),
    ("초과", "크다", false),
    ("미만", "작다", false),
];

pub const CALL_TAILS: &[&str] = &["값", "나머지"];

pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

pub fn particle(form: &str) -> Option<(&'static str, &'static str)> {
    PARTICLES
        .iter()
        .find(|&&(f, _, _)| f == form)
        .map(|&(_, role, canon)| (role, canon))
}
