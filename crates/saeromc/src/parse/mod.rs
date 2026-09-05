use crate::ast::*;
use crate::diag::{Diag, Result, Span};
use crate::hangul::{Ending, Pos};
use crate::lex::{tokenize, Num, Part, Tok, Token};
use crate::msg;
use crate::prescan::{resolve_module, Program};
use crate::sig::Marker;
use crate::words;
use std::path::Path;

const MAX_ERRORS: usize = 20;

pub struct Parsed {
    pub statements: Vec<Stmt>,
    pub errors: Vec<Diag>,
}

pub fn parse<'a>(
    tokens: &'a [Token],
    program: &'a Program,
    base_dir: Option<&'a Path>,
) -> Parsed {
    let mut parser = Parser {
        tokens,
        at: 0,
        program,
        base_dir,
        errors: Vec::new(),
        inside: false,
        plan: Vec::new(),
        picks: Vec::new(),
        stuck: false,
    };
    let statements = parser.program();
    Parsed {
        statements,
        errors: parser.errors,
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
    program: &'a Program,
    base_dir: Option<&'a Path>,
    errors: Vec<Diag>,
    inside: bool,
    // 용언마다 "몇 번째로 긴 묶기를 고를지". 되짚기가 이 벡터를 돌린다.
    plan: Vec<usize>,
    picks: Vec<usize>,
    stuck: bool,
}

#[derive(Clone)]
struct VerbInfo {
    name: String,
    pos: Pos,
    ending: Ending,
    negated: bool,
    span: Span,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &'a Tok {
        &self.ahead(0).tok
    }

    fn ahead(&self, offset: usize) -> &'a Token {
        let index = (self.at + offset).min(self.tokens.len() - 1);
        &self.tokens[index]
    }

    fn span(&self) -> Span {
        self.ahead(0).span
    }

    fn advance(&mut self) -> &'a Token {
        let token = self.ahead(0);
        self.at += 1;
        token
    }

    fn at_symbol(&self, offset: usize, ch: char) -> bool {
        self.ahead(offset).tok == Tok::Symbol(ch)
    }

    fn accept(&mut self, wanted: &Tok) -> bool {
        if self.peek() == wanted {
            self.at += 1;
            return true;
        }
        false
    }

    fn expect(&mut self, wanted: &Tok, what: &str) -> Result<&'a Token> {
        if self.peek() == wanted {
            return Ok(self.advance());
        }
        let token = self.ahead(0);
        if matches!(token.tok, Tok::Newline | Tok::Eof) {
            return Err(Diag::syntax(msg::line_ended(what), token.span));
        }
        Err(Diag::syntax(
            msg::not_wanted(what, &describe(&token.tok)),
            token.span,
        ))
    }

    fn expect_name(&mut self) -> Result<(String, Span)> {
        match self.peek() {
            Tok::Name(name) => {
                let name = name.clone();
                Ok((name, self.advance().span))
            }
            other => Err(Diag::syntax(msg::not_a_name(&describe(other)), self.span())),
        }
    }

    fn expect_particle(&mut self) -> Result<&'static str> {
        match self.peek() {
            Tok::Particle { canon, .. } => {
                let canon = *canon;
                self.at += 1;
                Ok(canon)
            }
            other => Err(Diag::syntax(
                msg::not_a_particle(&describe(other)),
                self.span(),
            )),
        }
    }

    fn line_end(&self) -> usize {
        let mut index = self.at;
        while index < self.tokens.len() && self.tokens[index].tok != Tok::Newline {
            index += 1;
        }
        index
    }

    fn resync(&mut self) {
        // 들여쓰기 경계는 넘지 않는다. 넘으면 다음 줄까지 통째로 먹는다.
        while !matches!(self.peek(), Tok::Eof | Tok::Indent(_) | Tok::Dedent(_)) {
            let done = matches!(self.peek(), Tok::Newline);
            self.at += 1;
            if done {
                break;
            }
        }
        let mut depth = 0usize;
        loop {
            match self.peek() {
                Tok::Indent(_) => depth += 1,
                Tok::Dedent(_) if depth > 0 => depth -= 1,
                Tok::Eof => return,
                _ if depth == 0 => return,
                _ => {}
            }
            self.at += 1;
        }
    }

    fn note(&mut self, error: Diag) {
        if self.errors.len() < MAX_ERRORS {
            self.errors.push(error);
        }
    }

    fn program(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();
        while !matches!(self.peek(), Tok::Eof) {
            if self.accept(&Tok::Newline) {
                continue;
            }
            match self.replanned(Self::statement) {
                Ok(statement) => statements.push(statement),
                Err(error) => {
                    self.note(error);
                    self.resync();
                }
            }
        }
        statements
    }

    // 묶기를 잘못 고르면 뒤에서야 드러난다. 걸리면 다른 묶기로 다시 읽는다.
    fn replanned<T>(&mut self, run: fn(&mut Self) -> Result<T>) -> Result<T> {
        let (start, errors, inside) = (self.at, self.errors.len(), self.inside);
        // 안쪽 되짚기가 바깥 것의 상태를 건드리지 않게 따로 둔다.
        let held = (
            std::mem::take(&mut self.plan),
            std::mem::take(&mut self.picks),
            self.stuck,
        );
        let made = self.replan_pass(run, start, errors, inside);
        (self.plan, self.picks, self.stuck) = held;
        made
    }

    fn replan_pass<T>(
        &mut self,
        run: fn(&mut Self) -> Result<T>,
        start: usize,
        errors: usize,
        inside: bool,
    ) -> Result<T> {
        let mut plan: Vec<usize> = Vec::new();
        for _ in 0..32 {
            self.at = start;
            self.errors.truncate(errors);
            self.inside = inside;
            self.plan.clone_from(&plan);
            self.picks.clear();
            self.stuck = false;
            let made = run(self);
            if made.is_ok() && !self.stuck {
                return made;
            }
            let counts = std::mem::take(&mut self.picks);
            if !bump(&mut plan, &counts) {
                break;
            }
        }
        self.at = start;
        self.errors.truncate(errors);
        self.inside = inside;
        self.plan.clear();
        self.picks.clear();
        self.stuck = false;
        run(self)
    }


    fn block(&mut self) -> Result<Block> {
        self.expect(&Tok::Symbol(':'), msg::WANT_COLON)?;
        self.expect(&Tok::Newline, msg::WANT_NEWLINE)?;
        if !matches!(self.peek(), Tok::Indent(_)) {
            return Err(Diag::syntax(msg::NO_BLOCK, self.span()));
        }
        self.at += 1;
        let mut statements = Vec::new();
        loop {
            match self.peek() {
                Tok::Dedent(_) | Tok::Eof => break,
                Tok::Newline => {
                    self.at += 1;
                }
                _ => match self.replanned(Self::statement) {
                    Ok(statement) => statements.push(statement),
                    Err(error) => {
                        self.note(error);
                        self.resync();
                    }
                },
            }
        }
        if matches!(self.peek(), Tok::Dedent(_)) {
            self.at += 1;
        }
        Ok(statements)
    }
}


// 계수기처럼 한 칸 올린다. 더 돌릴 데가 없으면 거짓.
fn bump(plan: &mut Vec<usize>, counts: &[usize]) -> bool {
    plan.resize(counts.len(), 0);
    for at in (0..counts.len()).rev() {
        if plan[at] + 1 < counts[at] {
            plan[at] += 1;
            plan[at + 1..].fill(0);
            return true;
        }
    }
    false
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Name(name) => format!("{} '{name}'", msg::TOK_NAME),
        Tok::Verb { name, .. } => format!("{} '{name}'", msg::TOK_VERB),
        Tok::Copula { .. } => msg::TOK_COPULA.into(),
        Tok::Particle { canon, .. } => format!("{} '{canon}'", msg::TOK_PARTICLE),
        Tok::Keyword(word) => format!("{} '{word}'", msg::TOK_KEYWORD),
        Tok::Number(Num::Int(value)) => format!("{} '{value}'", msg::TOK_NUMBER),
        Tok::Number(Num::Float(value)) => format!("{} '{value}'", msg::TOK_NUMBER),
        Tok::Str(_) | Tok::Template(_) => msg::TOK_STRING.into(),
        Tok::Symbol(ch) => format!("{} '{ch}'", msg::TOK_SYMBOL),
        Tok::Indent(_) => msg::TOK_INDENT.into(),
        Tok::Dedent(_) => msg::TOK_DEDENT.into(),
        Tok::Newline => msg::TOK_NEWLINE.into(),
        Tok::Eof => msg::TOK_EOF.into(),
    }
}

fn starts_value(tok: &Tok) -> bool {
    match tok {
        Tok::Number(_) | Tok::Str(_) | Tok::Template(_) | Tok::Name(_) => true,
        Tok::Keyword(word) => {
            matches!(word.as_str(), "참" | "거짓" | "없음" | "묶음")
        }
        Tok::Symbol(ch) => *ch == '(',
        _ => false,
    }
}

fn ending_label(ending: Ending) -> &'static str {
    match ending {
        Ending::Final => "-ㄴ다",
        Ending::AdnominalPast => "-ㄴ",
        Ending::AdnominalPres => "-는",
        Ending::Conditional => "-면",
        Ending::Conjunctive => "-고",
        Ending::Alternative => "-거나",
        Ending::Interrogative => "-ㄴ지",
        Ending::Auxiliary => msg::END_AUXILIARY,
        Ending::Negative => "-지",
        Ending::Quotative => "-라는",
    }
}

fn join_and(left: Option<Expr>, right: Expr, span: Span) -> Expr {
    match left {
        None => right,
        Some(before) => Expr::And {
            left: Box::new(before),
            right: Box::new(right),
            span,
        },
    }
}

fn join_or(apart: Vec<Expr>, span: Span) -> Expr {
    apart
        .into_iter()
        .reduce(|before, next| Expr::Or {
            left: Box::new(before),
            right: Box::new(next),
            span,
        })
        .expect("조건")
}

fn carry_subject(slots: &mut Vec<Slot>, subject: &mut Option<Expr>) {
    if !slots.iter().any(|slot| slot.marker == Marker::Case("가")) {
        if let Some(found) = subject.clone() {
            slots.insert(
                0,
                Slot {
                    marker: Marker::Case("가"),
                    expr: found,
                },
            );
        }
    }
    for slot in slots.iter() {
        if slot.marker == Marker::Case("가") {
            *subject = Some(slot.expr.clone());
        }
    }
}

fn copula_slots(mut slots: Vec<Slot>) -> Vec<Slot> {
    let subjects: Vec<usize> = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.marker == Marker::Case("가"))
        .map(|(index, _)| index)
        .collect();
    if let (true, Some(&last)) = (subjects.len() >= 2, subjects.last()) {
        slots[last].marker = Marker::Bare;
    }
    slots
}

fn fold_comparison(slots: Vec<Slot>, info: VerbInfo) -> (Vec<Slot>, VerbInfo) {
    if info.name != "이다" || slots.len() < 2 {
        return (slots, info);
    }
    let marker = slots[slots.len() - 1].marker;
    if !matches!(marker, Marker::Bare | Marker::Case("가"))
        || slots[slots.len() - 2].marker != Marker::Bare
    {
        return (slots, info);
    }
    let Some(word) = slots[slots.len() - 1].expr.as_name() else {
        return (slots, info);
    };
    let Some(&(_, verb, negated)) = words::COMPARATIVES
        .iter()
        .find(|&&(name, _, _)| name == word)
    else {
        return (slots, info);
    };
    let mut slots = slots;
    slots.pop();
    let operand = slots.pop().expect("견줄 값").expr;
    slots.push(Slot {
        marker: Marker::Case("보다"),
        expr: operand,
    });
    let info = VerbInfo {
        name: verb.into(),
        negated: negated != info.negated,
        ..info
    };
    (slots, info)
}
mod expr;
mod stmt;

fn not_dictionary_form(span: Span) -> Diag {
    Diag::syntax(msg::HEAD_NOT_DICT, span).with_hint(msg::HEAD_NOT_DICT_HELP)
}

