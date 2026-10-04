//! 낱말 하나가 무엇인지. 설명·정의·쓰인 곳·의미 강조가 여기서 갈린다.

use super::index::{is_field, Def, Index, Var};
use crate::hir;
use crate::lex::{Tok, Token};
use crate::sig::Marker;
use crate::types::{Ty, Types};
use crate::{builtins, words};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub enum Thing<'a> {
    Def(&'a Def),
    Var(&'a Var),
    Verb(String),
    Module(String, PathBuf),
    Field(&'static str),
    Key(String),
}

impl Thing<'_> {
    pub fn same(&self, other: &Thing) -> bool {
        match (self, other) {
            (Thing::Def(a), Thing::Def(b)) => std::ptr::eq(*a, *b),
            (Thing::Var(a), Thing::Var(b)) => std::ptr::eq(*a, *b),
            (Thing::Verb(a), Thing::Verb(b)) | (Thing::Key(a), Thing::Key(b)) => a == b,
            (Thing::Module(a, _), Thing::Module(b, _)) => a == b,
            (Thing::Field(a), Thing::Field(b)) => a == b,
            _ => false,
        }
    }
}

pub fn identify(index: &Index, at: usize) -> Option<Thing<'_>> {
    let token = &index.tokens[at];
    let line = token.span.line;
    let owned = at
        .checked_sub(1)
        .map(|before| &index.tokens[before])
        .filter(|before| before.span.line == line)
        .is_some_and(|before| matches!(before.tok, Tok::Particle { canon: "의", .. }));
    match &token.tok {
        Tok::Verb { name, .. } => Some(
            index
                .var(name, line)
                .map(Thing::Var)
                .or_else(|| index.def(name).map(Thing::Def))
                .unwrap_or_else(|| Thing::Verb(name.clone())),
        ),
        Tok::Name(name) if owned => {
            if let Some(field) = words::FIELDS.iter().find(|field| *field == name) {
                return Some(Thing::Field(field));
            }
            Some(match index.def(name) {
                Some(def) => Thing::Def(def),
                None => Thing::Key(name.clone()),
            })
        }
        Tok::Name(name) => {
            if let Some(var) = index.var(name, line) {
                return Some(Thing::Var(var));
            }
            if let Some(def) = index.def(name) {
                return Some(Thing::Def(def));
            }
            if let Some(path) = index.module(name) {
                return Some(Thing::Module(name.clone(), path.to_path_buf()));
            }
            words::FIELDS
                .iter()
                .find(|field| *field == name)
                .map(|field| Thing::Field(field))
        }
        _ => None,
    }
}

// 마지막으로 검사를 통과한 글의 추론 결과.
#[derive(Default)]
pub struct Typing {
    globals: HashMap<String, Ty>,
    // (정의 줄, 이름)
    locals: HashMap<(usize, String), Ty>,
    returns: HashMap<usize, Ty>,
}

impl Typing {
    pub fn new(program: &hir::Program, types: &Types) -> Typing {
        let mut found = Typing::default();
        let root = &program.modules[program.root as usize];
        for (name, slot) in &root.globals {
            found
                .globals
                .insert(name.to_string(), types.globals[*slot as usize]);
        }
        for (id, function) in program.functions.iter().enumerate() {
            if function.module != program.root {
                continue;
            }
            let line = function.span.line;
            for (name, slot) in &function.names {
                let ty = types.locals[id][*slot as usize];
                join(&mut found.locals, (line, name.to_string()), ty);
            }
            join(&mut found.returns, line, types.returns[id]);
        }
        found
    }

    fn var(&self, var: &Var) -> Option<&'static str> {
        let ty = match var.lines {
            None => self.globals.get(&var.name),
            Some((line, _)) => self.locals.get(&(line, var.name.clone())),
        };
        label(*ty?)
    }
}

// 특수화된 사본들은 합쳐 본다.
fn join<K: std::hash::Hash + Eq>(map: &mut HashMap<K, Ty>, key: K, ty: Ty) {
    let joined = map.get(&key).map_or(ty, |was| was.join(ty));
    map.insert(key, joined);
}

fn label(ty: Ty) -> Option<&'static str> {
    Some(match ty {
        Ty::Nothing => "없음",
        Ty::Bool => "논리값",
        Ty::Int => "정수",
        Ty::Float => "실수",
        Ty::Str => "문자열",
        Ty::Table => "묶음",
        Ty::Any | Ty::Never => return None,
    })
}

pub fn hover(thing: &Thing, typing: &Typing) -> String {
    match thing {
        Thing::Def(def) => {
            let mut out = code(&def.shape());
            let made = def
                .path
                .is_none()
                .then(|| typing.returns.get(&def.span.line))
                .flatten()
                .and_then(|&ty| label(ty));
            if let Some(made) = made {
                out.push_str(&format!("\n→ `{made}`"));
            }
            if !def.doc.is_empty() {
                out.push_str(&format!("\n\n{}", def.doc));
            }
            if let Some(path) = &def.path {
                out.push_str(&format!("\n\n`{}`", stem(path)));
            }
            out
        }
        Thing::Var(var) => {
            let mut out = format!("{}\n{}", code(&var.name), var.role.label());
            if let Some(ty) = typing.var(var) {
                out.push_str(&format!(" `{ty}`"));
            }
            out
        }
        Thing::Verb(verb) => {
            let note = builtin_note(verb);
            let mut out = code(&builtin_shape(verb));
            if !note.is_empty() {
                out.push_str(&format!("\n{note}"));
            }
            out
        }
        Thing::Module(name, path) => format!("{}\n`{}`", code(name), path.display()),
        Thing::Field(name) => format!("{}\n{}", code(name), field_note(name)),
        Thing::Key(name) => format!("{}\n명칭", code(name)),
    }
}

fn code(text: &str) -> String {
    format!("```saerom\n{text}\n```")
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| crate::hangul::to_nfc(&stem.to_string_lossy()))
        .unwrap_or_default()
}

pub fn builtin_shape(verb: &str) -> String {
    let mut found = builtins::ways(verb);
    let table = crate::sig::Signatures::builtin();
    if found.is_empty() {
        found = table.ways(verb).iter().map(Vec::as_slice).collect();
    }
    let ways: Vec<String> = found
        .iter()
        .map(|params| {
            let mut out = String::new();
            for marker in params.iter() {
                if *marker != Marker::Bare {
                    out.push_str(&format!("~{} ", marker.label()));
                }
            }
            out + verb
        })
        .collect();
    if ways.is_empty() {
        verb.to_string()
    } else {
        ways.join("\n")
    }
}

pub fn builtin_note(verb: &str) -> &'static str {
    match verb {
        "종료하다" => "종료 코드 1. 수를 주면 그 코드로 조용히 종료",
        "복사하다" => "묶음의 깊은 복사",
        "정렬하다" => "오름차순 정렬한 새 묶음. `<동사>로`를 주면 그 결과를 기준으로",
        "바꾸다" => "형변환. 실패하면 `없음`",
        "더하다" => "수, 문자열",
        "추가하다" => "마지막 자리에 추가",
        "제거하다" => "`<묶음>의 <자리|명칭>를 제거한다`",
        "삽입하다" => "`<묶음>의 <자리>에 <값>을 삽입한다`",
        "나누다" => "실수. `나눈 몫` `나눈 나머지`",
        "같다" => "내용을 비교",
        "열다" => "방식: `\"읽기\"` `\"쓰기\"` `\"추가\"`. 실패하면 `없음`",
        "읽다" => "바이트 수만큼. 끝이면 `없음`",
        "쓰다" => "쓴 바이트 수",
        _ => "",
    }
}

pub fn field_note(name: &str) -> &'static str {
    match name {
        "자료형" => "값의 자료형",
        "길이" => "묶음, 문자열. 문자열은 글자 단위",
        "명칭" => "명칭을 담은 묶음",
        "제곱근" => "수의 제곱근",
        _ => "",
    }
}

// 모듈 글의 이름표. 정의와 전역이 내보내진다.
pub fn module_index(path: &Path) -> Index {
    std::fs::read_to_string(path)
        .map(|source| super::index::build(&source, Some(path)))
        .unwrap_or_default()
}

// 같은 것을 가리키는 이 글의 낱말들.
pub fn occurrences(index: &Index, at: usize) -> Vec<&Token> {
    let Some(target) = identify(index, at) else {
        return Vec::new();
    };
    (0..index.tokens.len())
        .filter(|&other| identify(index, other).is_some_and(|found| found.same(&target)))
        .map(|other| &index.tokens[other])
        .collect()
}

// LSP SemanticTokenTypes 순서
pub const TYPES: &[&str] = &[
    "function",
    "variable",
    "parameter",
    "property",
    "keyword",
    "namespace",
    "particle",
    "copula",
];
pub const MODIFIERS: &[&str] = &["declaration", "defaultLibrary"];

pub fn classify(index: &Index, at: usize) -> Option<(usize, usize)> {
    const LIBRARY: usize = 2;
    let token = &index.tokens[at];
    let kind = |name| TYPES.iter().position(|found| *found == name).unwrap();
    Some(match &token.tok {
        Tok::Particle { .. } => (kind("particle"), 0),
        Tok::Copula { .. } => (kind("copula"), 0),
        Tok::Name(name) if name == "것" => (kind("keyword"), 0),
        Tok::Name(name) if words::CALL_TAILS.contains(&name.as_str()) => {
            (kind("property"), LIBRARY)
        }
        Tok::Name(name)
            if name.ends_with("번째") || words::COMPARATIVES.iter().any(|c| c.0 == name) =>
        {
            return None
        }
        Tok::Verb { .. } | Tok::Name(_) => match identify(index, at) {
            Some(Thing::Def(def)) => {
                let declaring = def.path.is_none() && def.lines.0 == token.span.line;
                let kind = if def.noun() {
                    kind("property")
                } else {
                    kind("function")
                };
                (kind, usize::from(declaring))
            }
            Some(Thing::Var(var)) if var.name.ends_with('다') => (kind("parameter"), 0),
            Some(Thing::Var(var)) => {
                let kind = if var.role == super::index::Role::Param {
                    kind("parameter")
                } else {
                    kind("variable")
                };
                (kind, 0)
            }
            Some(Thing::Verb(_)) => (kind("function"), LIBRARY),
            Some(Thing::Module(..)) => (kind("namespace"), 0),
            Some(Thing::Field(_)) => (kind("property"), LIBRARY),
            Some(Thing::Key(_)) => (kind("property"), 0),
            None if is_field(name_of(&token.tok)) => (kind("property"), LIBRARY),
            None => (kind("variable"), 0),
        },
        _ => return None,
    })
}

fn name_of(tok: &Tok) -> &str {
    match tok {
        Tok::Name(name) => name,
        _ => "",
    }
}

// 정의 줄에서 이름이 실제로 놓인 자리.
pub fn pin<'a>(index: &'a Index, line: usize, name: &str) -> Option<&'a Token> {
    index.tokens.iter().find(|token| {
        token.span.line == line && matches!(&token.tok, Tok::Name(found) if found == name)
    })
}

// 가져온 모듈의 정의 이름 자리. 그 글을 다시 훑는다.
pub fn foreign_pin(path: &Path, def: &Def) -> crate::diag::Span {
    let Ok(source) = std::fs::read_to_string(path) else {
        return def.span;
    };
    let built = super::index::build(&source, Some(path));
    pin(&built, def.span.line, &def.name).map_or(def.span, |token| token.span)
}
