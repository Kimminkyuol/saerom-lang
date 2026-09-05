use crate::msg;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Marker {
    Case(&'static str),
    Bare,
    Module,
}

impl Marker {
    pub fn is_argument(self) -> bool {
        match self {
            Marker::Case(particle) => !crate::words::STRUCTURAL.contains(&particle),
            Marker::Bare => true,
            Marker::Module => false,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Marker::Case(particle) => particle,
            Marker::Bare => "없음",
            Marker::Module => "모듈",
        }
    }
}

pub type Signature = Vec<Marker>;

// 조사 다발이 같은가.
pub fn same(left: &[Marker], right: &[Marker]) -> bool {
    left.len() == right.len() && ordered(left.iter().copied()) == ordered(right.iter().copied())
}

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

    // 내장 이름은 예약이다. 사용자 정의가 이기면 `3에 4를 더한 값`이 조용히
    // 남의 함수로 간다. 시그니처가 달라도 막는다 — 제거하다처럼 이름만으로
    // 특수 하강되는 것이 있어 시그니처 비교로는 새는 구멍이 남는다.
    pub fn reserved(verb: &str) -> bool {
        static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
        NAMES
            .get_or_init(|| Signatures::builtin().0.keys().cloned().collect())
            .iter()
            .any(|name| name == verb)
    }

    pub fn knows(&self, verb: &str) -> bool {
        self.0.contains_key(verb)
    }

}

const BUILTIN: &[(&str, &[Marker])] = &[
    ("출력하다", &[Marker::Case("를")]),
    ("추가하다", &[Marker::Case("에"), Marker::Case("를")]),
    ("제거하다", &[Marker::Case("를")]),
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
    ("반환하다", &[Marker::Case("를")]),
    ("이다", &[Marker::Case("가"), Marker::Bare]),
    ("이다", &[Marker::Case("가")]),
    ("이다", &[Marker::Bare]),
    ("아니다", &[Marker::Case("가"), Marker::Bare]),
    ("아니다", &[Marker::Case("가")]),
];

pub fn shown(markers: &[Marker]) -> String {
    let used: Vec<&str> = markers
        .iter()
        .filter(|m| **m != Marker::Bare)
        .map(|m| m.label())
        .collect();
    if used.is_empty() {
        msg::NO_PARTICLE.to_string()
    } else {
        used.join(", ")
    }
}

pub fn describe(verb: &str, params: &[Marker]) -> String {
    let mut out = String::new();
    for marker in params {
        if *marker != Marker::Bare {
            out.push_str(&format!("~{} ", marker.label()));
        }
    }
    out.push_str(verb);
    out
}
