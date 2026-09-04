const BASE: u32 = 0xAC00;
const LAST: u32 = 0xD7A3;

const ONSETS: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ',
    'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];
const VOWELS: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ',
    'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];
const CODAS: [char; 27] = [
    'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ',
    'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pos {
    Verb,
    Descriptive,
    Passive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    Final,
    AdnominalPast,
    AdnominalPres,
    Conditional,
    Conjunctive,
    Alternative,
    Interrogative,
    Auxiliary,
    Negative,
    Quotative,
}

pub const REGULAR_ENDINGS: [Ending; 9] = [
    Ending::Final,
    Ending::AdnominalPast,
    Ending::AdnominalPres,
    Ending::Conditional,
    Ending::Conjunctive,
    Ending::Alternative,
    Ending::Interrogative,
    Ending::Auxiliary,
    Ending::Negative,
];

pub fn is_syllable(ch: char) -> bool {
    (BASE..=LAST).contains(&(ch as u32))
}

pub fn decompose(ch: char) -> Option<(char, char, Option<char>)> {
    if !is_syllable(ch) {
        return None;
    }
    let code = ch as u32 - BASE;
    let coda = code % 28;
    Some((
        ONSETS[(code / 588) as usize],
        VOWELS[((code % 588) / 28) as usize],
        (coda > 0).then(|| CODAS[(coda - 1) as usize]),
    ))
}

pub fn compose(onset: char, vowel: char, coda: Option<char>) -> char {
    let onset = ONSETS.iter().position(|&c| c == onset).unwrap_or(0) as u32;
    let vowel = VOWELS.iter().position(|&c| c == vowel).unwrap_or(0) as u32;
    let coda = coda
        .and_then(|c| CODAS.iter().position(|&k| k == c))
        .map_or(0, |index| index as u32 + 1);
    char::from_u32(BASE + onset * 588 + vowel * 28 + coda).unwrap_or('?')
}

pub fn coda_of(ch: char) -> Option<char> {
    decompose(ch).and_then(|(_, _, coda)| coda)
}

fn drop_coda(ch: char) -> char {
    match decompose(ch) {
        Some((onset, vowel, _)) => compose(onset, vowel, None),
        None => ch,
    }
}

fn add_coda(ch: char, coda: char) -> char {
    match decompose(ch) {
        Some((onset, vowel, None)) => compose(onset, vowel, Some(coda)),
        _ => ch,
    }
}

pub fn conjugate(stem: &str, pos: Pos, ending: Ending) -> Option<String> {
    let last = stem.chars().last()?;
    let head = &stem[..stem.len() - last.len_utf8()];
    let coda = coda_of(last);
    let open = coda.is_none();
    let riul = coda == Some('ㄹ');
    let descriptive = pos == Pos::Descriptive;
    let nieun = || {
        let base = if riul { drop_coda(last) } else { last };
        format!("{head}{}", add_coda(base, 'ㄴ'))
    };

    Some(match ending {
        Ending::Final if descriptive => format!("{stem}다"),
        Ending::Final if riul || open => format!("{}다", nieun()),
        Ending::Final => format!("{stem}는다"),

        Ending::AdnominalPast if riul || open => nieun(),
        Ending::AdnominalPast => format!("{stem}은"),

        Ending::AdnominalPres if descriptive => {
            return conjugate(stem, pos, Ending::AdnominalPast)
        }
        Ending::AdnominalPres if riul => format!("{head}{}는", drop_coda(last)),
        Ending::AdnominalPres => format!("{stem}는"),

        Ending::Conditional if open || riul => format!("{stem}면"),
        Ending::Conditional => format!("{stem}으면"),

        Ending::Conjunctive => format!("{stem}고"),
        Ending::Alternative => format!("{stem}거나"),
        Ending::Negative => format!("{stem}지"),

        Ending::Interrogative => {
            let base = if descriptive {
                Ending::AdnominalPast
            } else {
                Ending::AdnominalPres
            };
            format!("{}지", conjugate(stem, pos, base)?)
        }

        Ending::Auxiliary => auxiliary(stem, head, last)?,

        Ending::Quotative => return None,
    })
}

// ㅅ/ㄷ/ㅂ 불규칙 대체형. 어간이 불규칙인지 알 수 없으니 둘 다 받아 준다.
pub fn irregular(stem: &str, ending: Ending) -> Option<String> {
    let last = stem.chars().last()?;
    let head = &stem[..stem.len() - last.len_utf8()];
    let (onset, vowel, coda) = decompose(last)?;
    let bare = compose(onset, vowel, None);
    let bright = matches!(vowel, 'ㅏ' | 'ㅗ');
    Some(match (coda?, ending) {
        // 짓 → 지은 / 지으면 / 지어
        ('ㅅ', Ending::AdnominalPast) => format!("{head}{bare}은"),
        ('ㅅ', Ending::Conditional) => format!("{head}{bare}으면"),
        ('ㅅ', Ending::Auxiliary) => {
            format!("{head}{bare}{}", if bright { "아" } else { "어" })
        }
        // 듣 → 들은 / 들으면 / 들어
        ('ㄷ', Ending::AdnominalPast) => format!("{head}{}은", add_coda(bare, 'ㄹ')),
        ('ㄷ', Ending::Conditional) => format!("{head}{}으면", add_coda(bare, 'ㄹ')),
        ('ㄷ', Ending::Auxiliary) => {
            format!("{head}{}{}", add_coda(bare, 'ㄹ'), if bright { "아" } else { "어" })
        }
        // 돕 → 도운 / 도우면 / 도와
        ('ㅂ', Ending::AdnominalPast) => {
            format!("{head}{bare}{}", add_coda('우', 'ㄴ'))
        }
        ('ㅂ', Ending::Conditional) => format!("{head}{bare}우면"),
        ('ㅂ', Ending::Auxiliary) => {
            format!("{head}{bare}{}", if bright { "와" } else { "워" })
        }
        _ => return None,
    })
}

fn auxiliary(stem: &str, head: &str, last: char) -> Option<String> {
    if last == '하' {
        return Some(format!("{head}해"));
    }
    let (onset, vowel, coda) = decompose(last)?;
    if coda.is_some() {
        return Some(format!(
            "{stem}{}",
            if matches!(vowel, 'ㅏ' | 'ㅗ') {
                "아"
            } else {
                "어"
            }
        ));
    }
    Some(match vowel {
        'ㅏ' | 'ㅓ' | 'ㅐ' | 'ㅔ' => stem.to_string(),
        'ㅗ' => format!("{head}{}", compose(onset, 'ㅘ', None)),
        'ㅜ' => format!("{head}{}", compose(onset, 'ㅝ', None)),
        'ㅡ' => format!("{head}{}", compose(onset, 'ㅓ', None)),
        'ㅣ' => format!("{head}{}", compose(onset, 'ㅕ', None)),
        _ => format!("{stem}어"),
    })
}

pub fn to_nfc(source: &str) -> String {
    const L: u32 = 0x1100;
    const V: u32 = 0x1161;
    const T: u32 = 0x11A8;
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        let code = ch as u32;
        if !(L..L + 19).contains(&code) {
            out.push(ch);
            continue;
        }
        let Some(vowel) = chars
            .peek()
            .map(|&c| c as u32)
            .filter(|v| (V..V + 21).contains(v))
        else {
            out.push(ch);
            continue;
        };
        chars.next();
        let mut syllable = BASE + (code - L) * 588 + (vowel - V) * 28;
        if let Some(coda) = chars
            .peek()
            .map(|&c| c as u32)
            .filter(|t| (T..T + 27).contains(t))
        {
            chars.next();
            syllable += coda - T + 1;
        }
        out.push(char::from_u32(syllable).unwrap_or(ch));
    }
    out
}

impl Ending {
    pub fn as_str(self) -> &'static str {
        match self {
            Ending::Final => "final",
            Ending::AdnominalPast => "adnominal_past",
            Ending::AdnominalPres => "adnominal_pres",
            Ending::Conditional => "conditional",
            Ending::Conjunctive => "conjunctive",
            Ending::Alternative => "alternative",
            Ending::Interrogative => "interrogative",
            Ending::Auxiliary => "auxiliary",
            Ending::Negative => "negative",
            Ending::Quotative => "quotative",
        }
    }
}

impl Pos {
    pub fn as_str(self) -> &'static str {
        match self {
            Pos::Verb => "verb",
            Pos::Descriptive => "descriptive",
            Pos::Passive => "passive",
        }
    }
}

pub fn subject_particle(word: &str) -> &'static str {
    match word.chars().last() {
        Some(ch) if coda_of(ch).is_some() => "이",
        _ => "가",
    }
}
