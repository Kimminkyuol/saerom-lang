//! 편집 중인 글 하나의 이름표.

use crate::ast::{Expr, LoopKind, Selector, Stmt};
use crate::diag::Span;
use crate::lex::{tokenize, Part, Tok, Token};
use crate::load;
use crate::msg;
use crate::prescan::{self, Program};
use crate::sig::Marker;
use crate::words;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Def {
    pub name: String,
    pub params: Vec<(Marker, String)>,
    pub span: Span,
    // 없으면 이 글.
    pub path: Option<PathBuf>,
    pub doc: String,
    pub lines: (usize, usize),
}

impl Def {
    pub fn noun(&self) -> bool {
        !self.name.ends_with('다')
    }

    pub fn shape(&self) -> String {
        let mut out = String::new();
        for (marker, name) in &self.params {
            out.push_str(&format!("{name}{} ", attach(name, marker.label())));
        }
        out.push_str(&self.name);
        out
    }
}

// 이름 끝 받침에 맞는 조사 꼴.
fn attach(name: &str, canon: &'static str) -> &'static str {
    let coda = name.chars().last().and_then(crate::hangul::coda_of);
    let closed = coda.is_some();
    match canon {
        "는" if closed => "은",
        "가" if closed => "이",
        "를" if closed => "을",
        "와" if closed => "과",
        "로" if closed && coda != Some('ㄹ') => "으로",
        _ => canon,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Global,
    Local,
    Param,
    Loop,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::Global => "전역 변수",
            Role::Local => "지역 변수",
            Role::Param => "매개변수",
            Role::Loop => "반복 변수",
        }
    }
}

pub struct Var {
    pub name: String,
    pub span: Span,
    pub role: Role,
    pub lines: Option<(usize, usize)>,
}

impl Var {
    pub fn seen_at(&self, line: usize) -> bool {
        self.lines
            .is_none_or(|(start, end)| (start..=end).contains(&line))
    }
}

#[derive(Default)]
pub struct Index {
    pub program: Program,
    // 글 조각 안의 식까지 펼친 낱말.
    pub tokens: Vec<Token>,
    pub defs: Vec<Def>,
    pub vars: Vec<Var>,
    pub keys: HashMap<String, Vec<String>>,
    pub modules: Vec<(String, PathBuf)>,
}

pub fn build(text: &str, path: Option<&Path>) -> Index {
    let base = path.and_then(Path::parent);
    let Some((fixed, program)) = lenient(text, base) else {
        return Index::default();
    };
    let mut index = Index {
        tokens: expand(&program),
        program,
        ..Index::default()
    };
    let Ok(loaded) = load::load(&fixed, path) else {
        return index;
    };
    for (id, unit) in loaded.units.iter().enumerate() {
        let mine = id == loaded.root;
        let lines: Vec<&str> = unit.source.lines().collect();
        let scopes = scopes(&unit.statements);
        for (statement, scope) in unit.statements.iter().zip(&scopes) {
            let (name, params, span) = match statement {
                Stmt::Define {
                    name, params, span, ..
                } => (name, params.clone(), *span),
                Stmt::Noun {
                    name, owner, span, ..
                } => (name, vec![(Marker::Case("의"), owner.clone())], *span),
                Stmt::Import { module, path, .. } if mine => {
                    index.modules.push((module.clone(), path.clone()));
                    continue;
                }
                _ => continue,
            };
            index.defs.push(Def {
                name: name.clone(),
                params,
                span,
                path: if mine { None } else { unit.path.clone() },
                doc: comment_above(&lines, span.line),
                lines: *scope,
            });
        }
        if mine {
            for (statement, scope) in unit.statements.iter().zip(&scopes) {
                let inside = matches!(statement, Stmt::Define { .. } | Stmt::Noun { .. });
                index.walk(std::slice::from_ref(statement), inside.then_some(*scope));
            }
        }
    }
    // 이 글의 정의가 가져온 것보다 앞선다.
    index.defs.sort_by_key(|def| def.path.is_some());
    index
}

impl Index {
    pub fn def(&self, name: &str) -> Option<&Def> {
        self.defs.iter().find(|def| def.name == name)
    }

    // 정의 안의 이름이 전역보다 앞선다.
    pub fn var(&self, name: &str, line: usize) -> Option<&Var> {
        let named = || self.vars.iter().filter(move |var| var.name == name);
        named()
            .find(|var| var.lines.is_some() && var.seen_at(line))
            .or_else(|| named().find(|var| var.lines.is_none()))
    }

    pub fn module(&self, name: &str) -> Option<&Path> {
        self.modules
            .iter()
            .find(|(module, _)| module == name)
            .map(|(_, path)| path.as_path())
    }

    // 사전형이 아니어도 동사로 쓰는 이름. 동사 자리 매개변수.
    pub fn verbs(&self, line: usize) -> Vec<String> {
        let mut found: Vec<String> = self
            .program
            .signatures
            .iter()
            .map(|(verb, _)| verb.to_string())
            .filter(|verb| !matches!(verb.as_str(), "이다" | "아니다"))
            .collect();
        found.extend(
            self.vars
                .iter()
                .filter(|var| var.role == Role::Param && var.name.ends_with('다'))
                .filter(|var| var.seen_at(line))
                .map(|var| var.name.clone()),
        );
        found.sort();
        found.dedup();
        found
    }

    pub fn at(&self, line: usize, col: usize) -> Option<(usize, &Token)> {
        // 글 조각과 그 안의 낱말이 겹치면 안쪽.
        self.tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| {
                token.span.line == line && token.span.col <= col && col <= token.span.end
            })
            .min_by_key(|(_, token)| token.span.end - token.span.col)
    }

    fn walk(&mut self, block: &[Stmt], scope: Option<(usize, usize)>) {
        let local = if scope.is_some() {
            Role::Local
        } else {
            Role::Global
        };
        for statement in block {
            match statement {
                Stmt::Declare { assigns, .. } => {
                    for (target, value) in assigns {
                        match target.fields.as_slice() {
                            [] => {
                                self.keep(&target.root, target.span, local, scope);
                                if let Expr::Table { entries, .. } = value {
                                    for (key, _) in entries {
                                        self.key(&target.root, key);
                                    }
                                }
                            }
                            [Selector::Name(key)] => self.key(&target.root, key),
                            _ => {}
                        }
                    }
                }
                Stmt::If {
                    branches,
                    otherwise,
                    ..
                } => {
                    for (_, body) in branches {
                        self.walk(body, scope);
                    }
                    if let Some(body) = otherwise {
                        self.walk(body, scope);
                    }
                }
                Stmt::Loop { kind, body, span } => {
                    if let LoopKind::Range { variable, .. } | LoopKind::Each { variable, .. } =
                        kind
                    {
                        self.keep(variable, *span, Role::Loop, scope);
                    }
                    self.walk(body, scope);
                }
                Stmt::Define {
                    params, body, span, ..
                } => {
                    for (_, name) in params {
                        self.keep(name, *span, Role::Param, scope);
                    }
                    self.walk(body, scope);
                }
                Stmt::Noun {
                    owner, body, span, ..
                } => {
                    self.keep(owner, *span, Role::Param, scope);
                    self.walk(body, scope);
                }
                _ => {}
            }
        }
    }

    fn keep(&mut self, name: &str, span: Span, role: Role, lines: Option<(usize, usize)>) {
        if self
            .vars
            .iter()
            .any(|var| var.name == name && var.lines == lines)
        {
            return;
        }
        self.vars.push(Var {
            name: name.to_string(),
            span,
            role,
            lines,
        });
    }

    fn key(&mut self, owner: &str, key: &str) {
        let keys = self.keys.entry(owner.to_string()).or_default();
        if !keys.iter().any(|known| known == key) {
            keys.push(key.to_string());
        }
    }
}

// 최상위 문장마다 다음 최상위 문장 전까지의 줄.
fn scopes(statements: &[Stmt]) -> Vec<(usize, usize)> {
    statements
        .iter()
        .enumerate()
        .map(|(index, statement)| {
            let end = statements
                .get(index + 1)
                .map_or(usize::MAX, |next| next.span().line.saturating_sub(1));
            (statement.span().line, end)
        })
        .collect()
}

fn comment_above(lines: &[&str], line: usize) -> String {
    let mut found = Vec::new();
    for text in lines[..line.saturating_sub(1).min(lines.len())]
        .iter()
        .rev()
    {
        let Some(rest) = text.trim().strip_prefix('#') else {
            break;
        };
        found.push(rest.trim());
    }
    found.reverse();
    found.join("\n")
}

// 편집 중에는 마침표 하나 빠진 줄이 흔하다. 낱말 분석이 막히는 줄을
// 고치거나 비워 가며 나머지를 살린다.
fn lenient(text: &str, base: Option<&Path>) -> Option<(String, Program)> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    for _ in 0..64 {
        let joined = lines.join("\n");
        let error = match prescan::survey(&joined, base) {
            Ok(program) => return Some((joined, program)),
            Err(error) => error,
        };
        let at = error
            .span
            .line
            .checked_sub(1)
            .filter(|&at| at < lines.len())?;
        let before = (0..at).rev().find(|&i| {
            let text = lines[i].trim();
            !text.is_empty() && !text.starts_with('#')
        });
        match before {
            Some(i) if error.msg == msg::MISSING_PERIOD && !lines[i].contains(['#', '"']) => {
                lines[i].push_str(" .");
            }
            _ => lines[at].clear(),
        }
    }
    None
}

fn expand(program: &Program) -> Vec<Token> {
    let mut out = Vec::new();
    for token in &program.tokens {
        match &token.tok {
            Tok::Template(parts) => {
                for part in parts {
                    let Part::Expr { source, span } = part else {
                        continue;
                    };
                    let Ok(inner) = tokenize(source, &program.vocab) else {
                        continue;
                    };
                    out.extend(inner.into_iter().filter(|t| real(&t.tok)).map(|mut t| {
                        t.span =
                            Span::new(span.line, t.span.col + span.col, t.span.end + span.col);
                        t
                    }));
                }
                out.push(token.clone());
            }
            tok if real(tok) => out.push(token.clone()),
            _ => {}
        }
    }
    out.sort_by_key(|token| (token.span.line, token.span.col));
    out
}

fn real(tok: &Tok) -> bool {
    !matches!(
        tok,
        Tok::Newline | Tok::Eof | Tok::Indent(_) | Tok::Dedent(_)
    )
}

pub fn modules_near(base: Option<&Path>) -> Vec<String> {
    let mut found = Vec::new();
    for folder in base
        .map(Path::to_path_buf)
        .into_iter()
        .chain(prescan::std_dirs())
    {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = crate::hangul::to_nfc(&entry.file_name().to_string_lossy());
            if let Some(stem) = name.strip_suffix(".sr") {
                found.push(stem.to_string());
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

pub fn is_field(name: &str) -> bool {
    words::FIELDS.contains(&name)
}
