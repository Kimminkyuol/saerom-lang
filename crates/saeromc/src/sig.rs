use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Marker {
    Case(&'static str),
    Bare,
    Module,
    Step,
}

impl Marker {
    pub fn is_argument(self) -> bool {
        match self {
            Marker::Case(particle) => !crate::words::STRUCTURAL.contains(&particle),
            Marker::Bare => true,
            Marker::Module | Marker::Step => false,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Marker::Case(particle) => particle,
            Marker::Bare => "없음",
            Marker::Module => "모듈",
            Marker::Step => "간격",
        }
    }
}

pub type Signature = Vec<Marker>;

pub fn ordered(markers: impl IntoIterator<Item = Marker>) -> Signature {
    let mut found: Signature = markers.into_iter().collect();
    found.sort_unstable();
    found
}

const MAX_SLOTS: usize = 8;

pub fn fits(used: &[Marker], signature: &[Marker]) -> bool {
    if used.len() > signature.len() || signature.len() > MAX_SLOTS {
        return false;
    }
    let mut taken = [false; MAX_SLOTS];
    'next: for marker in used {
        for (index, kept) in signature.iter().enumerate() {
            if !taken[index] && kept == marker {
                taken[index] = true;
                continue 'next;
            }
        }
        return false;
    }
    true
}

#[derive(Default, Clone, Debug)]
pub struct Signatures(HashMap<String, Vec<Signature>>);

impl Signatures {
    pub fn builtin() -> Self {
        let mut table = Signatures::default();
        for &(verb, markers) in BUILTIN {
            table.add(verb, ordered(markers.iter().copied()));
        }
        table
    }

    pub fn add(&mut self, verb: &str, signature: Signature) {
        let ways = self.0.entry(verb.to_string()).or_default();
        if !ways.contains(&signature) {
            ways.push(signature);
        }
    }

    pub fn ways(&self, verb: &str) -> &[Signature] {
        self.0.get(verb).map_or(&[], Vec::as_slice)
    }

    pub fn knows(&self, verb: &str) -> bool {
        self.0.contains_key(verb)
    }

    pub fn absorb(&mut self, other: &Signatures) {
        for (verb, ways) in &other.0 {
            for signature in ways {
                self.add(verb, signature.clone());
            }
        }
    }
}

const BUILTIN: &[(&str, &[Marker])] = &[
    ("출력하다", &[Marker::Case("를")]),
    ("종료하다", &[Marker::Case("로")]),
    ("복사하다", &[Marker::Case("를")]),
    ("바꾸다", &[Marker::Case("를"), Marker::Case("로")]),
    ("더하다", &[Marker::Case("에"), Marker::Case("를")]),
    ("빼다", &[Marker::Case("에서"), Marker::Case("를")]),
    ("곱하다", &[Marker::Case("에"), Marker::Case("를")]),
    ("나누다", &[Marker::Case("를"), Marker::Case("로")]),
    ("크다", &[Marker::Case("가"), Marker::Case("보다")]),
    ("작다", &[Marker::Case("가"), Marker::Case("보다")]),
    ("같다", &[Marker::Case("가"), Marker::Case("와")]),
    ("같다", &[Marker::Case("가"), Marker::Case("보다")]),
    ("읽다", &[Marker::Case("에서"), Marker::Case("만큼")]),
    ("열다", &[Marker::Case("를"), Marker::Case("로")]),
    ("닫다", &[Marker::Case("를")]),
    ("쓰다", &[Marker::Case("에"), Marker::Case("를")]),
    ("가져오다", &[Marker::Case("를")]),
    ("돌려주다", &[Marker::Case("를")]),
    ("이다", &[Marker::Case("가"), Marker::Bare]),
    ("이다", &[Marker::Case("가")]),
    ("이다", &[Marker::Bare]),
    ("아니다", &[Marker::Case("가"), Marker::Bare]),
    ("아니다", &[Marker::Case("가")]),
];
