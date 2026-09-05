use crate::sig::Marker;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Print,
    Stop,
    Clone,
    Convert,
    Add,
    Push,
    RemoveAt,
    RemoveKey,
    Sub,
    Mul,
    Div,
    Quot,
    Rem,
    Greater,
    Less,
    Equal,
    Truthy,
    Read,
    Close,
    Open,
    Write,
    Nothing,
}

pub struct Def {
    pub verb: &'static str,
    pub params: &'static [Marker],
    pub op: Builtin,
}

const CASE: fn(&'static str) -> Marker = Marker::Case;

pub fn table() -> &'static [Def] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<Vec<Def>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let one = |verb, particle, op| Def {
            verb,
            params: leak(vec![CASE(particle)]),
            op,
        };
        let two = |verb, first, second, op| Def {
            verb,
            params: leak(vec![CASE(first), CASE(second)]),
            op,
        };
        vec![
            one("출력하다", "를", Builtin::Print),
            two("추가하다", "에", "를", Builtin::Push),
            one("종료하다", "로", Builtin::Stop),
            one("복사하다", "를", Builtin::Clone),
            two("바꾸다", "를", "로", Builtin::Convert),
            two("더하다", "에", "를", Builtin::Add),
            two("빼다", "에서", "를", Builtin::Sub),
            two("곱하다", "에", "를", Builtin::Mul),
            two("나누다", "를", "로", Builtin::Div),
            two("나누다·나머지", "를", "로", Builtin::Rem),
            two("나누다·몫", "를", "로", Builtin::Quot),
            two("크다", "가", "보다", Builtin::Greater),
            two("작다", "가", "보다", Builtin::Less),
            two("같다", "가", "와", Builtin::Equal),
            two("읽다", "에서", "만큼", Builtin::Read),
            two("열다", "를", "로", Builtin::Open),
            one("닫다", "를", Builtin::Close),
            two("쓰다", "에", "를", Builtin::Write),
            one("가져오다", "를", Builtin::Nothing),
            Def {
                verb: "이다",
                params: leak(vec![CASE("가"), Marker::Bare]),
                op: Builtin::Equal,
            },
            Def {
                verb: "이다",
                params: leak(vec![CASE("가")]),
                op: Builtin::Truthy,
            },
            Def {
                verb: "이다",
                params: leak(vec![Marker::Bare]),
                op: Builtin::Truthy,
            },
        ]
    })
}

fn leak(markers: Vec<Marker>) -> &'static [Marker] {
    Vec::leak(markers)
}

pub fn named(op: Builtin) -> &'static str {
    table()
        .iter()
        .find(|def| def.op == op)
        .map_or("내장", |def| def.verb)
}

pub fn find(verb: &str, used: &[Marker]) -> Option<&'static Def> {
    table().iter().find(|def| {
        def.verb == verb && crate::sig::fits(used, def.params) && used.len() == def.params.len()
    })
}

pub fn ways(verb: &str) -> Vec<&'static [Marker]> {
    table()
        .iter()
        .filter(|def| def.verb == verb)
        .map(|def| def.params)
        .collect()
}
