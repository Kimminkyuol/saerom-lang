//! 자동완성. 줄의 앞부분을 낱말로 나눠 자리를 짐작한다.

use super::index::{modules_near, Index, Role};
use super::json::{obj, Json};
use super::look::{self, builtin_note, field_note};
use crate::hangul::Ending;
use crate::lex::{tokenize, Tok, Token};
use crate::sig::{self, Marker};
use crate::words;
use std::path::Path;

// LSP CompletionItemKind
const FUNCTION: usize = 3;
const FIELD: usize = 5;
const VARIABLE: usize = 6;
const MODULE: usize = 9;
const PROPERTY: usize = 10;
const KEYWORD: usize = 14;
const SNIPPET: usize = 15;
const CONSTANT: usize = 21;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Statement,
    Condition,
    Value,
}

#[derive(Clone, Copy)]
enum Form {
    End(Ending),
    Called,
    Denied,
}

const STATEMENT: &[Form] = &[
    Form::End(Ending::Final),
    Form::End(Ending::Conjunctive),
    Form::Called,
    Form::End(Ending::Conditional),
    Form::End(Ending::Interrogative),
    Form::End(Ending::Alternative),
    Form::Denied,
];
const CONDITION: &[Form] = &[
    Form::End(Ending::Conditional),
    Form::End(Ending::Conjunctive),
    Form::End(Ending::Alternative),
    Form::Denied,
    Form::End(Ending::Interrogative),
    Form::Called,
    Form::End(Ending::Final),
];
const VALUE: &[Form] = &[
    Form::Called,
    Form::End(Ending::Interrogative),
    Form::End(Ending::Conjunctive),
    Form::End(Ending::Final),
    Form::End(Ending::Conditional),
    Form::End(Ending::Alternative),
    Form::Denied,
];

pub struct Items(Vec<Json>);

impl Items {
    fn add(&mut self, label: &str, kind: usize, sort: String, detail: &str, doc: &str) {
        self.0.push(obj([
            ("label", label.into()),
            ("kind", kind.into()),
            ("sortText", sort.into()),
            ("detail", (!detail.is_empty()).then_some(detail).into()),
            ("documentation", markdown(doc)),
        ]));
    }

    fn snippet(&mut self, label: &str, body: &str, sort: String, detail: &str) {
        self.0.push(obj([
            ("label", label.into()),
            ("kind", SNIPPET.into()),
            ("sortText", sort.into()),
            ("detail", detail.into()),
            ("insertText", body.into()),
            ("insertTextFormat", 2usize.into()),
        ]));
    }
}

fn markdown(text: &str) -> Json {
    if text.is_empty() {
        return Json::Null;
    }
    obj([("kind", "markdown".into()), ("value", text.into())])
}

pub fn complete(
    index: &Index,
    prefix: &str,
    line: usize,
    path: Option<&Path>,
    typed: bool,
) -> Json {
    let base = path.and_then(Path::parent);
    let Some(code) = code_part(prefix) else {
        return Json::Null;
    };
    let word = code
        .chars()
        .rev()
        .take_while(|&c| c.is_alphanumeric() || c == '_')
        .count();
    let cut = code
        .char_indices()
        .rev()
        .nth(word.wrapping_sub(1))
        .map_or(code.len(), |(at, _)| at);
    let before = &code[..if word == 0 { code.len() } else { cut }];
    let tokens: Vec<Token> = tokenize(before, &index.program.vocab)
        .map(|found| {
            found
                .into_iter()
                .filter(|t| {
                    !matches!(
                        t.tok,
                        Tok::Newline | Tok::Eof | Tok::Indent(_) | Tok::Dedent(_)
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let mut items = Items(Vec::new());

    if let [.., owner, Token {
        tok: Tok::Particle { canon: "의", .. },
        ..
    }] = tokens.as_slice()
    {
        fields(index, &owner.tok, &mut items);
        return Json::Arr(items.0);
    }
    if let Some(module) = importing(index, &tokens, base) {
        let module = look::module_index(&module);
        for def in module.defs.iter().filter(|def| def.path.is_none()) {
            let kind = if def.noun() { PROPERTY } else { FUNCTION };
            items.add(
                &def.name,
                kind,
                format!("0{}", def.name),
                &def.shape(),
                &def.doc,
            );
        }
        globals(&module, &mut items);
        items.add("가져온다.", KEYWORD, "1".into(), "", "");
        return Json::Arr(items.0);
    }
    // 띄어쓰기로 불렸으면 확실한 자리에서만 띄운다.
    if typed {
        return Json::Null;
    }
    general(
        index,
        &tokens,
        line,
        path,
        before.trim().is_empty() && !before.starts_with(' '),
        &mut items,
    );
    Json::Arr(items.0)
}

// 주석이면 없음. 글 조각 `{ }` 안이면 그 식만.
fn code_part(prefix: &str) -> Option<&str> {
    let mut quoted = false;
    let mut brace: Option<usize> = None;
    let mut escaped = false;
    for (at, ch) in prefix.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if quoted && brace.is_none() => escaped = true,
            '"' if brace.is_none() => quoted = !quoted,
            '{' if quoted => brace = Some(at + 1),
            '}' if quoted => brace = None,
            '#' if !quoted => return None,
            _ => {}
        }
    }
    match (quoted, brace) {
        (false, _) => Some(prefix),
        (true, Some(at)) => Some(&prefix[at..]),
        (true, None) => None,
    }
}

fn fields(index: &Index, owner: &Tok, items: &mut Items) {
    let owner = match owner {
        Tok::Name(owner) => owner.as_str(),
        _ => "",
    };
    // `<모듈>의 <이름>`
    if let Some(path) = index.module(owner) {
        let module = look::module_index(path);
        for def in module.defs.iter().filter(|def| def.path.is_none()) {
            if def.noun() {
                items.add(
                    &def.name,
                    PROPERTY,
                    format!("1{}", def.name),
                    &def.shape(),
                    &def.doc,
                );
            } else {
                verbs(&module, &def.name, &[], Mode::Value, items);
            }
        }
        globals(&module, items);
        return;
    }
    for key in index.keys.get(owner).into_iter().flatten() {
        items.add(key, FIELD, format!("0{key}"), "명칭", "");
    }
    for &name in words::FIELDS {
        items.add(name, PROPERTY, format!("1{name}"), "", field_note(name));
    }
    items.snippet("번째", "${1:1}번째", "1번째".into(), "자리");
    for name in &index.program.nouns {
        let (shape, doc) = index
            .def(name)
            .map(|def| (def.shape(), def.doc.as_str()))
            .unwrap_or_default();
        items.add(name, PROPERTY, format!("2{name}"), &shape, doc);
    }
}

fn globals(module: &Index, items: &mut Items) {
    for var in module.vars.iter().filter(|var| var.lines.is_none()) {
        items.add(
            &var.name,
            VARIABLE,
            format!("0{}", var.name),
            var.role.label(),
            "",
        );
    }
}

// `<모듈>에서 ...` 를 쓰는 중.
fn importing(
    index: &Index,
    tokens: &[Token],
    base: Option<&Path>,
) -> Option<std::path::PathBuf> {
    let [Token {
        tok: Tok::Name(module),
        ..
    }, Token {
        tok: Tok::Particle {
            canon: "에서", ..
        },
        ..
    }, rest @ ..] = tokens
    else {
        return None;
    };
    let listing = rest
        .iter()
        .all(|token| matches!(&token.tok, Tok::Name(_) | Tok::Particle { canon: "와", .. }));
    if !listing || index.vars.iter().any(|var| &var.name == module) {
        return None;
    }
    crate::prescan::resolve_module(module, base)
}

fn general(
    index: &Index,
    tokens: &[Token],
    line: usize,
    path: Option<&Path>,
    top: bool,
    items: &mut Items,
) {
    let start = tokens.is_empty();
    let mode = match tokens.first().map(|t| &t.tok) {
        Some(Tok::Keyword(word)) if word == "만약" || word == "아니고" => Mode::Condition,
        _ if matches!(
            tokens.get(1).map(|t| &t.tok),
            Some(Tok::Particle { canon: "는", .. })
        ) =>
        {
            Mode::Value
        }
        _ => Mode::Statement,
    };
    let used = clause(tokens);
    let looping = used.contains(&Marker::Case("마다"))
        || tokens.iter().any(|t| t.tok == Tok::Keyword("동안".into()));
    let used: Vec<Marker> = used
        .into_iter()
        .filter(|marker| marker.is_argument() && *marker != Marker::Case("는"))
        .collect();

    if looping {
        items.add("반복한다:", KEYWORD, "00".into(), "", "");
    }
    for verb in index.verbs(line) {
        verbs(index, &verb, &used, mode, items);
    }
    // 문장 첫머리에는 동사보다 이름이 온다.
    if start {
        for item in &mut items.0 {
            if let Json::Obj(pairs) = item {
                for (key, value) in pairs.iter_mut() {
                    if let (true, Json::Str(sort)) = (key == "sortText", value) {
                        sort.insert(0, '6');
                    }
                }
            }
        }
    }

    let mut seen = Vec::new();
    for var in &index.vars {
        if !var.seen_at(line) || var.name.ends_with('다') || seen.contains(&&var.name) {
            continue;
        }
        seen.push(&var.name);
        let rank = if var.role == Role::Global { 2 } else { 1 };
        items.add(
            &var.name,
            VARIABLE,
            format!("{rank}{}", var.name),
            var.role.label(),
            "",
        );
    }
    for module in &index.program.modules {
        items.add(module, MODULE, format!("3{module}"), "모듈", "");
    }
    for name in ["정수", "실수", "문자열", "논리값"] {
        items.add(name, CONSTANT, format!("4{name}"), "자료형", "");
    }
    items.add("실행인자", VARIABLE, "4실행인자".into(), "명령줄 인자", "");
    for word in ["참", "거짓", "없음", "묶음", "이상", "이하", "초과", "미만"] {
        items.add(word, KEYWORD, format!("4{word}"), "", "");
    }
    if mode == Mode::Condition {
        for word in ["그리고", "또는"] {
            items.add(word, KEYWORD, format!("3{word}"), "", "");
        }
    }
    if start {
        statements(index, line, top, path, items);
    }
}

fn statements(index: &Index, line: usize, top: bool, path: Option<&Path>, items: &mut Items) {
    let base = path.and_then(Path::parent);
    let here = path
        .and_then(Path::file_stem)
        .map(|stem| crate::hangul::to_nfc(&stem.to_string_lossy()));
    let here = here.as_deref();
    items.snippet("만약", "만약 ${1:조건}면:\n\t$0", "3만약".into(), "조건문");
    items.snippet(
        "아니고 만약",
        "아니고 만약 ${1:조건}면:\n\t$0",
        "3아니고".into(),
        "",
    );
    items.snippet("아니면", "아니면:\n\t$0", "3아니면".into(), "");
    let inside = index.defs.iter().any(|def| {
        def.path.is_none() && (def.lines.0..=def.lines.1).contains(&line) && def.lines.0 != line
    });
    if inside {
        items.snippet("반환한다", "${1:값}을 반환한다.", "3반환".into(), "");
    }
    for word in ["빠져나간다.", "넘어간다."] {
        items.add(word, KEYWORD, format!("4{word}"), "", "");
    }
    if !top {
        return;
    }
    for module in modules_near(base) {
        if index.module(&module).is_some() || here == Some(module.as_str()) {
            continue;
        }
        items.add(
            &format!("{module}을 가져온다."),
            MODULE,
            format!("5{module}"),
            "모듈",
            "",
        );
    }
}

// 마지막 용언 뒤로 쓴 조사들.
fn clause(tokens: &[Token]) -> Vec<Marker> {
    let mut used = Vec::new();
    for token in tokens {
        match &token.tok {
            Tok::Verb { .. } | Tok::Copula { .. } | Tok::Symbol(_) => used.clear(),
            Tok::Keyword(word) if word == "그리고" || word == "또는" => used.clear(),
            Tok::Particle { canon, .. } => used.push(Marker::Case(canon)),
            _ => {}
        }
    }
    used
}

fn verbs(index: &Index, verb: &str, used: &[Marker], mode: Mode, items: &mut Items) {
    let ways = index.program.signatures.ways(verb);
    let fit = if used.is_empty() || ways.is_empty() {
        1
    } else if ways.iter().any(|way| sig::fits(used, way)) {
        0
    } else {
        2
    };
    let (detail, doc) = match index.def(verb) {
        Some(def) => (def.shape(), def.doc.clone()),
        None => (look::builtin_shape(verb), builtin_note(verb).to_string()),
    };
    // `X가 <이름>이다` 정의는 이다로 활용한다.
    let copular = verb.len() > "이다".len()
        && verb.ends_with("이다")
        && ways
            .iter()
            .all(|way| way.as_slice() == [Marker::Case("가")]);
    let forms: Vec<(Ending, String)> = if copular {
        let head = &verb[..verb.len() - "이다".len()];
        words::COPULA
            .iter()
            .filter(|(_, ending)| *ending != Ending::Quotative)
            .map(|&(form, ending)| (ending, format!("{head}{form}")))
            .collect()
    } else {
        words::forms_of(verb)
    };
    let surface = |ending| {
        forms
            .iter()
            .find(|(found, _)| *found == ending)
            .map(|(_, form)| form.clone())
    };
    let order = match mode {
        Mode::Statement => STATEMENT,
        Mode::Condition => CONDITION,
        Mode::Value => VALUE,
    };
    for (rank, form) in order.iter().enumerate() {
        let labels: Vec<String> = match form {
            Form::End(ending) => surface(*ending).into_iter().collect(),
            Form::Called if copular => Vec::new(),
            Form::Called => surface(Ending::AdnominalPast)
                .into_iter()
                .flat_map(|stem| {
                    let tails: &[&str] = if verb == "나누다" {
                        &["값", "몫", "나머지"]
                    } else {
                        &["값"]
                    };
                    tails.iter().map(move |tail| format!("{stem} {tail}"))
                })
                .collect(),
            Form::Denied => surface(Ending::Negative)
                .map(|stem| {
                    let tail = if mode == Mode::Condition {
                        "않으면"
                    } else {
                        "않은지"
                    };
                    format!("{stem} {tail}")
                })
                .into_iter()
                .collect(),
        };
        for label in labels {
            items.add(
                &label,
                FUNCTION,
                format!("{fit}{rank}{label}"),
                &detail,
                &doc,
            );
        }
    }
}
