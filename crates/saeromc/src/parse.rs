use crate::ast::*;
use crate::diag::{Diag, Result, Span};
use crate::hangul::{Ending, Pos};
use crate::lex::{tokenize, Num, Part, Tok, Token};
use crate::msg;
use crate::prescan::{resolve_module, Program};
use crate::sig::{fits, Marker};
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
        while !matches!(self.peek(), Tok::Eof) {
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
            match self.statement() {
                Ok(statement) => statements.push(statement),
                Err(error) => {
                    self.note(error);
                    self.resync();
                }
            }
        }
        statements
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
                _ => match self.statement() {
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
            matches!(word.as_str(), "참" | "거짓" | "없음" | "목록" | "사전")
        }
        Tok::Symbol(ch) => *ch == '[' || *ch == '{',
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

impl<'a> Parser<'a> {
    fn take_verb(&mut self) -> Result<VerbInfo> {
        let token = self.ahead(0);
        let (name, pos, ending) = match &token.tok {
            Tok::Verb { name, pos, ending } => (name.clone(), *pos, *ending),
            Tok::Copula { ending } => ("이다".to_string(), Pos::Descriptive, *ending),
            other => return Err(Diag::syntax(msg::not_a_verb(&describe(other)), token.span)),
        };
        self.at += 1;
        if name == "아니다" {
            let span = token.span;
            return Ok(VerbInfo {
                name: "이다".into(),
                pos: Pos::Descriptive,
                ending,
                negated: true,
                span,
            });
        }
        if ending == Ending::Negative {
            let partner = self.ahead(0);
            let Tok::Verb {
                name: helper,
                ending: after,
                ..
            } = &partner.tok
            else {
                return Err(Diag::syntax(msg::NOT_NEGATION, partner.span));
            };
            if helper != "않다" {
                return Err(Diag::syntax(msg::NOT_NEGATION, partner.span));
            }
            let after = *after;
            self.at += 1;
            return Ok(VerbInfo {
                name,
                pos,
                ending: after,
                negated: true,
                span: token.span,
            });
        }
        Ok(VerbInfo {
            name,
            pos,
            ending,
            negated: false,
            span: token.span,
        })
    }

    fn starts_predicate(&self, offset: usize) -> bool {
        let Tok::Name(name) = &self.ahead(offset).tok else {
            return false;
        };
        matches!(self.ahead(offset + 1).tok, Tok::Copula { .. })
            && self.program.signatures.knows(&format!("{name}이다"))
    }

    fn chain(&mut self, value: Expr) -> Expr {
        let mut value = value;
        loop {
            if !matches!(self.peek(), Tok::Particle { canon: "의", .. }) {
                return value;
            }
            let Tok::Name(field) = &self.ahead(1).tok else {
                return value;
            };
            if self.program.modules.contains(field) {
                return value;
            }
            if let Tok::Copula { ending } = self.ahead(2).tok {
                if ending != Ending::Final {
                    return value;
                }
            }
            let span = self.ahead(1).span;
            let name = field.clone();
            self.at += 2;
            value = Expr::Field {
                owner: Box::new(value),
                name,
                span,
            };
        }
    }

    fn push(&mut self, slots: &mut Vec<Slot>, first: Expr) -> Result<()> {
        let mut items = vec![self.chain(first)];
        while matches!(self.peek(), Tok::Particle { role: "conj", .. })
            && starts_value(&self.ahead(1).tok)
            && !self.starts_predicate(1)
        {
            self.at += 1;
            let next = self.primary()?;
            items.push(self.chain(next));
        }
        let value = if items.len() == 1 {
            items.pop().expect("항 하나")
        } else {
            let span = items[0].span();
            Expr::List { items, span }
        };
        let mut marker = Marker::Bare;
        if let Tok::Particle { canon, .. } = self.peek() {
            let canon = *canon;
            let namespace = canon == "의"
                && value
                    .as_name()
                    .is_some_and(|name| self.program.modules.contains(name));
            self.at += 1;
            marker = if namespace {
                Marker::Module
            } else {
                Marker::Case(canon)
            };
        }
        slots.push(Slot {
            marker,
            expr: value,
        });
        Ok(())
    }

    fn split_slots(&self, verb: &str, slots: Vec<Slot>) -> (Vec<Slot>, Vec<Slot>) {
        let ways = self.program.signatures.ways(verb);
        if ways.is_empty() {
            return (Vec::new(), slots);
        }
        let (structural, arguments): (Vec<Slot>, Vec<Slot>) = slots
            .into_iter()
            .partition(|slot| !slot.marker.is_argument());
        let total = arguments.len();
        for count in (0..=total).rev() {
            let tail = &arguments[total - count..];
            let used: Vec<Marker> = tail.iter().map(|slot| slot.marker).collect();
            if ways.iter().any(|way| fits(&used, way)) {
                let mut taken = structural;
                taken.extend_from_slice(tail);
                return (arguments[..total - count].to_vec(), taken);
            }
        }
        (arguments, structural)
    }

    fn reduce(&mut self, slots: Vec<Slot>, info: VerbInfo) -> Result<(Expr, Vec<Slot>)> {
        let (kept, slots) = if info.name == "이다" || info.pos == Pos::Passive {
            (Vec::new(), slots)
        } else {
            self.split_slots(&info.name, slots)
        };
        let (mut slots, info) = if info.name == "이다" {
            fold_comparison(copula_slots(slots), info)
        } else {
            (slots, info)
        };

        let token = self.ahead(0);
        let follows_name = matches!(token.tok, Tok::Name(_));
        let tail = match &token.tok {
            Tok::Name(name) if words::CALL_TAILS.contains(&name.as_str()) => Some(name.clone()),
            _ => None,
        };

        if follows_name && tail.is_none() {
            let (head, span) = self.expect_name()?;
            if info.pos == Pos::Passive {
                let head = Expr::Name { name: head, span };
                let call = PassiveExpr {
                    verb: info.name,
                    head,
                    slots,
                    span: info.span,
                };
                return Ok((Expr::Passive(Box::new(call)), kept));
            }
            return Err(Diag::syntax(msg::not_head_value(&head), span));
        }
        if tail.is_some() {
            self.at += 1;
        }
        if info.pos == Pos::Passive {
            if let Some(index) = slots
                .iter()
                .position(|slot| slot.marker == Marker::Case("를"))
            {
                let head = slots.remove(index).expr;
                let call = PassiveExpr {
                    verb: info.name,
                    head,
                    slots,
                    span: info.span,
                };
                return Ok((Expr::Passive(Box::new(call)), kept));
            }
        }
        let call = CallExpr {
            verb: info.name,
            slots,
            negated: info.negated,
            asks: info.ending == Ending::Interrogative,
            tail: Some(tail.unwrap_or_else(|| "값".into())),
            span: info.span,
        };
        Ok((Expr::Call(Box::new(call)), kept))
    }

    fn primary(&mut self) -> Result<Expr> {
        let token = self.ahead(0);
        let span = token.span;
        match &token.tok {
            Tok::Number(Num::Int(value)) => {
                let value = *value;
                self.at += 1;
                Ok(Expr::Literal {
                    value: Literal::Int(value),
                    span,
                })
            }
            Tok::Number(Num::Float(value)) => {
                let value = *value;
                self.at += 1;
                Ok(Expr::Literal {
                    value: Literal::Float(value),
                    span,
                })
            }
            Tok::Str(text) => {
                let text = text.clone();
                self.at += 1;
                Ok(Expr::Literal {
                    value: Literal::Str(text),
                    span,
                })
            }
            Tok::Template(parts) => {
                let parts = parts.clone();
                self.at += 1;
                let mut made = Vec::new();
                for part in parts {
                    made.push(match part {
                        Part::Text(text) => TemplatePart::Text(text),
                        Part::Expr { source, span } => {
                            TemplatePart::Expr(self.fragment(&source, span)?)
                        }
                    });
                }
                Ok(Expr::Template { parts: made, span })
            }
            Tok::Keyword(word) if word == "참" || word == "거짓" => {
                let value = word == "참";
                self.at += 1;
                Ok(Expr::Literal {
                    value: Literal::Bool(value),
                    span,
                })
            }
            Tok::Keyword(word) if word == "없음" => {
                self.at += 1;
                Ok(Expr::Literal {
                    value: Literal::Nothing,
                    span,
                })
            }
            Tok::Keyword(word) if word == "목록" => {
                self.at += 1;
                Ok(Expr::List {
                    items: Vec::new(),
                    span,
                })
            }
            Tok::Keyword(word) if word == "사전" => {
                self.at += 1;
                Ok(Expr::Dict {
                    entries: Vec::new(),
                    span,
                })
            }
            Tok::Name(name) => {
                let name = name.clone();
                self.at += 1;
                Ok(Expr::Name { name, span })
            }
            other => Err(Diag::syntax(msg::not_a_value(&describe(other)), span)),
        }
    }

    fn reduce_until(&mut self, stop: fn(&Tok) -> bool, what: &str) -> Result<Expr> {
        let mut slots: Vec<Slot> = Vec::new();
        loop {
            let token = self.ahead(0);
            if stop(&token.tok) || matches!(token.tok, Tok::Eof) {
                if slots.len() != 1 {
                    let mark = crate::hangul::subject_particle(what);
                    return Err(Diag::syntax(msg::not_one(what, mark), token.span));
                }
                return Ok(slots.pop().expect("값 하나").expr);
            }
            if matches!(token.tok, Tok::Verb { .. } | Tok::Copula { .. }) {
                let info = self.take_verb()?;
                let (value, kept) = self.reduce(slots, info)?;
                slots = kept;
                self.push(&mut slots, value)?;
                continue;
            }
            let value = self.primary()?;
            self.push(&mut slots, value)?;
        }
    }

    fn fragment(&mut self, source: &str, at: Span) -> Result<Expr> {
        let tokens = tokenize(source, &self.program.vocab)
            .map_err(|error| Diag { span: at, ..error })?;
        let shifted: Vec<Token> = tokens
            .into_iter()
            .map(|token| Token {
                tok: token.tok,
                span: Span::new(at.line, at.col + token.span.col, at.col + token.span.end),
            })
            .collect();
        let mut inner = Parser {
            tokens: &shifted,
            at: 0,
            program: self.program,
            base_dir: self.base_dir,
            errors: Vec::new(),
        };
        inner.reduce_until(
            |tok| matches!(tok, Tok::Newline | Tok::Eof),
            msg::WANT_EMBEDDED,
        )
    }
}

impl<'a> Parser<'a> {
    fn keyword_at(&self, offset: usize, word: &str) -> bool {
        matches!(&self.ahead(offset).tok, Tok::Keyword(found) if found == word)
    }

    fn accept_keyword(&mut self, word: &str) -> bool {
        if self.keyword_at(0, word) {
            self.at += 1;
            return true;
        }
        false
    }

    fn expect_keyword(&mut self, word: &str) -> Result<()> {
        if self.accept_keyword(word) {
            return Ok(());
        }
        Err(Diag::syntax(
            msg::not_keyword(word, &describe(self.peek())),
            self.span(),
        ))
    }

    fn expect_verb_named(&mut self, wanted: &str) -> Result<()> {
        if matches!(self.peek(), Tok::Verb { name, .. } if name == wanted) {
            self.at += 1;
            return Ok(());
        }
        Err(Diag::syntax(
            msg::not_keyword(wanted, &describe(self.peek())),
            self.span(),
        ))
    }

    fn end_of_statement(&mut self) -> Result<()> {
        self.expect(&Tok::Symbol('.'), msg::WANT_PERIOD)?;
        self.expect(&Tok::Newline, msg::WANT_NEWLINE)?;
        Ok(())
    }

    fn tok_at(&self, index: usize) -> &'a Tok {
        &self.tokens[index.min(self.tokens.len() - 1)].tok
    }

    fn statement(&mut self) -> Result<Stmt> {
        let token = self.ahead(0);
        if self.keyword_at(0, "만약") {
            return self.if_statement();
        }
        if self.looks_like_definition() {
            return self.definition();
        }
        let lone_value = matches!(token.tok, Tok::Number(_) | Tok::Str(_))
            || self.keyword_at(0, "참")
            || self.keyword_at(0, "거짓")
            || self.keyword_at(0, "없음");
        if lone_value && matches!(self.ahead(1).tok, Tok::Newline) {
            let expr = self.primary()?;
            self.expect(&Tok::Newline, msg::WANT_NEWLINE)?;
            return Ok(Stmt::Value {
                expr,
                span: token.span,
            });
        }
        if self.looks_like_import() {
            return self.import_statement();
        }
        if let Some(target) = self.try_target()? {
            let span = target.span;
            let value = self.value_until_copula()?;
            self.end_of_statement()?;
            return Ok(Stmt::Declare {
                target,
                value,
                span,
            });
        }
        self.exec_or_loop()
    }

    fn looks_like_definition(&self) -> bool {
        let end = self.line_end();
        if end < self.at + 3 {
            return false;
        }
        self.tok_at(end - 1) == &Tok::Symbol(':')
            && matches!(self.tok_at(end - 2), Tok::Particle { role: "topic", .. })
            && matches!(self.tok_at(end - 3), Tok::Name(_))
    }

    fn looks_like_import(&self) -> bool {
        let end = self.line_end();
        end >= self.at + 3
            && self.tok_at(end - 1) == &Tok::Symbol('.')
            && matches!(self.tok_at(end - 2), Tok::Verb { name, .. } if name == "가져오다")
    }

    fn ends_with_copula(&self) -> bool {
        let end = self.line_end();
        end >= self.at + 2
            && self.tok_at(end - 1) == &Tok::Symbol('.')
            && matches!(
                self.tok_at(end - 2),
                Tok::Copula {
                    ending: Ending::Final
                }
            )
    }

    fn try_target(&mut self) -> Result<Option<Target>> {
        let start = self.at;
        let Tok::Name(root) = self.tok_at(start) else {
            if let Tok::Keyword(word) = self.tok_at(start) {
                if matches!(self.tok_at(start + 1), Tok::Particle { role: "topic", .. }) {
                    return Err(Diag::syntax(msg::reserved_target(word), self.ahead(0).span));
                }
            }
            return Ok(None);
        };
        let mut fields = Vec::new();
        let mut index = start + 1;
        while matches!(self.tok_at(index), Tok::Particle { canon: "의", .. }) {
            let Tok::Name(field) = self.tok_at(index + 1) else {
                break;
            };
            fields.push(field.clone());
            index += 2;
        }
        let taken = match self.tok_at(index) {
            Tok::Particle { role: "topic", .. } => true,
            Tok::Particle {
                role: "subject", ..
            } => self.ends_with_copula(),
            _ => false,
        };
        if !taken {
            return Ok(None);
        }
        let span = self.ahead(0).span;
        self.at = index + 1;
        Ok(Some(Target {
            root: root.clone(),
            fields,
            span,
        }))
    }

    fn value_until_copula(&mut self) -> Result<Expr> {
        let mut slots: Vec<Slot> = Vec::new();
        loop {
            let token = self.ahead(0);
            if matches!(
                token.tok,
                Tok::Copula {
                    ending: Ending::Final
                }
            ) {
                self.at += 1;
                if slots.len() != 1 {
                    return Err(Diag::syntax(msg::DECL_NOT_ONE, token.span));
                }
                return Ok(slots.pop().expect("값 하나").expr);
            }
            if matches!(token.tok, Tok::Verb { .. } | Tok::Copula { .. }) {
                let info = self.take_verb()?;
                if !matches!(
                    info.ending,
                    Ending::AdnominalPast | Ending::AdnominalPres | Ending::Interrogative
                ) {
                    return Err(Diag::syntax(
                        msg::decl_bad_ending(ending_label(info.ending)),
                        info.span,
                    ));
                }
                let (value, kept) = self.reduce(slots, info)?;
                slots = kept;
                self.push(&mut slots, value)?;
                continue;
            }
            if matches!(token.tok, Tok::Newline | Tok::Eof) {
                return Err(Diag::syntax(msg::DECL_NO_COPULA, token.span));
            }
            let value = self.primary()?;
            self.push(&mut slots, value)?;
        }
    }

    fn if_statement(&mut self) -> Result<Stmt> {
        let span = self.span();
        self.expect_keyword("만약")?;
        let mut branches = vec![(self.condition()?, self.block()?)];
        let mut otherwise = None;
        loop {
            if self.accept_keyword("아니고") {
                self.expect_keyword("만약")?;
                branches.push((self.condition()?, self.block()?));
            } else if self.accept_keyword("아니면") {
                otherwise = Some(self.block()?);
                break;
            } else {
                break;
            }
        }
        Ok(Stmt::If {
            branches,
            otherwise,
            span,
        })
    }

    fn condition(&mut self) -> Result<Expr> {
        let mut left: Option<Expr> = None;
        let mut any = false;
        let mut subject: Option<Expr> = None;
        let mut slots: Vec<Slot> = Vec::new();
        loop {
            let token = self.ahead(0);
            let negated_else = self.keyword_at(0, "아니면") && !slots.is_empty();
            if !negated_else && !matches!(token.tok, Tok::Verb { .. } | Tok::Copula { .. }) {
                let value = self.primary()?;
                self.push(&mut slots, value)?;
                continue;
            }
            let info = if negated_else {
                self.at += 1;
                VerbInfo {
                    name: "이다".into(),
                    pos: Pos::Descriptive,
                    ending: Ending::Conditional,
                    negated: true,
                    span: token.span,
                }
            } else {
                self.take_verb()?
            };
            if matches!(info.ending, Ending::AdnominalPast | Ending::AdnominalPres) {
                let (value, kept) = self.reduce(slots, info)?;
                slots = kept;
                self.push(&mut slots, value)?;
                continue;
            }
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
            for slot in &slots {
                if slot.marker == Marker::Case("가") {
                    subject = Some(slot.expr.clone());
                }
            }
            let taken = std::mem::take(&mut slots);
            let (taken, info) = if info.name == "이다" {
                fold_comparison(copula_slots(taken), info)
            } else {
                (taken, info)
            };
            let span = info.span;
            let piece = Expr::Call(Box::new(CallExpr {
                verb: info.name,
                slots: taken,
                negated: info.negated,
                asks: false,
                tail: None,
                span,
            }));
            left = Some(match left {
                None => piece,
                Some(before) if any => Expr::Or {
                    left: Box::new(before),
                    right: Box::new(piece),
                    span,
                },
                Some(before) => Expr::And {
                    left: Box::new(before),
                    right: Box::new(piece),
                    span,
                },
            });
            match info.ending {
                Ending::Conditional => return Ok(left.expect("조건")),
                Ending::Conjunctive => any = false,
                Ending::Alternative => any = true,
                other => {
                    return Err(Diag::syntax(
                        msg::cond_bad_ending(ending_label(other)),
                        span,
                    ))
                }
            }
        }
    }

    fn definition(&mut self) -> Result<Stmt> {
        let span = self.span();
        let (head, params) = self.definition_head()?;
        let (thing, thing_span) = self.expect_name()?;
        if thing != "것" {
            return Err(Diag::syntax(msg::not_thing(&thing), thing_span));
        }
        self.expect_particle()?;
        if !head.ends_with('다') {
            if params.len() != 1 || params[0].0 != Marker::Case("의") {
                return Err(Diag::syntax(msg::noun_needs_owner(&head), span));
            }
            let owner = params[0].1.clone();
            let body = self.block()?;
            return Ok(Stmt::Noun {
                name: head,
                owner,
                body,
                span,
            });
        }
        let body = self.block()?;
        Ok(Stmt::Define {
            name: head,
            params,
            body,
            span,
        })
    }

    fn definition_head(&mut self) -> Result<(String, Vec<(Marker, String)>)> {
        let mut params: Vec<(Marker, String)> = Vec::new();
        loop {
            let token = self.ahead(0);
            let after = self.ahead(1);
            if let (Tok::Name(name), Tok::Copula { ending }) = (&token.tok, &after.tok) {
                if *ending != Ending::Quotative {
                    return Err(not_dictionary_form(token.span));
                }
                let name = name.clone();
                self.at += 2;
                return Ok((name, params));
            }
            if matches!(token.tok, Tok::Verb { .. } | Tok::Copula { .. }) {
                return Err(not_dictionary_form(token.span));
            }
            let Tok::Name(name) = &token.tok else {
                return Err(Diag::syntax(
                    msg::head_not_phrase(&describe(&token.tok)),
                    token.span,
                ));
            };
            let Tok::Particle { canon, .. } = &after.tok else {
                return Err(not_dictionary_form(token.span));
            };
            if self.at_symbol(2, ':') {
                return Err(Diag::syntax(msg::HEAD_NO_QUOTATIVE, token.span));
            }
            if self.tail_is_head(2) {
                return Err(not_dictionary_form(token.span));
            }
            let canon = *canon;
            if params
                .iter()
                .any(|(marker, _)| *marker == Marker::Case(canon))
            {
                return Err(Diag::syntax(msg::head_twice(canon), after.span));
            }
            params.push((Marker::Case(canon), name.clone()));
            self.at += 2;
        }
    }

    fn tail_is_head(&self, offset: usize) -> bool {
        matches!(self.ahead(offset).tok, Tok::Name(ref name) if name == "것")
            && matches!(
                self.ahead(offset + 1).tok,
                Tok::Particle { role: "topic", .. }
            )
            && self.at_symbol(offset + 2, ':')
    }

    fn import_name(&mut self) -> Result<String> {
        match self.peek() {
            Tok::Name(name) => {
                let name = name.clone();
                self.at += 1;
                if matches!(
                    self.peek(),
                    Tok::Copula {
                        ending: Ending::Final
                    }
                ) {
                    self.at += 1;
                    return Ok(format!("{name}이다"));
                }
                Ok(name)
            }
            Tok::Verb { name, .. } => {
                let name = name.clone();
                self.at += 1;
                Ok(name)
            }
            other => Err(Diag::syntax(
                msg::not_import_name(&describe(other)),
                self.span(),
            )),
        }
    }

    fn import_statement(&mut self) -> Result<Stmt> {
        let span = self.span();
        let module = self.import_name()?;
        let particle = self.expect_particle()?;
        let mut names = None;
        if particle == "에서" {
            let mut taken = vec![self.import_name()?];
            while matches!(self.peek(), Tok::Particle { role: "conj", .. }) {
                self.at += 1;
                taken.push(self.import_name()?);
            }
            self.expect_particle()?;
            names = Some(taken);
        }
        self.expect_verb_named("가져오다")?;
        self.end_of_statement()?;
        let Some(path) = resolve_module(&module, self.base_dir) else {
            return Err(Diag::syntax(msg::module_missing(&module), span));
        };
        Ok(Stmt::Import {
            module,
            names,
            path,
            span,
        })
    }

    fn exec_or_loop(&mut self) -> Result<Stmt> {
        let mut slots: Vec<Slot> = Vec::new();
        let mut calls: Vec<CallExpr> = Vec::new();
        loop {
            let token = self.ahead(0);
            if self.keyword_at(0, "간격") {
                self.at += 1;
                if matches!(self.peek(), Tok::Particle { canon: "의", .. }) {
                    self.at += 1;
                }
                let Some(last) = slots.last_mut() else {
                    return Err(Diag::syntax(msg::NO_STEP_NUMBER, token.span));
                };
                last.marker = Marker::Step;
                continue;
            }
            if self.keyword_at(0, "동안") {
                self.at += 1;
                self.expect_verb_named("반복하다")?;
                if slots.len() != 1 {
                    return Err(Diag::syntax(msg::WHILE_NOT_ONE, token.span));
                }
                let test = slots.pop().expect("조건").expr;
                let body = self.block()?;
                return Ok(Stmt::Loop {
                    kind: LoopKind::While { test },
                    body,
                    span: token.span,
                });
            }
            if !matches!(token.tok, Tok::Verb { .. } | Tok::Copula { .. }) {
                let value = self.primary()?;
                self.push(&mut slots, value)?;
                continue;
            }

            let info = self.take_verb()?;
            if info.ending == Ending::Final {
                match info.name.as_str() {
                    "반복하다" => return self.range_loop(slots, info.span),
                    "빠져나가다" | "넘어가다" => {
                        self.end_of_statement()?;
                        let span = info.span;
                        return Ok(if info.name == "빠져나가다" {
                            Stmt::Break { span }
                        } else {
                            Stmt::Continue { span }
                        });
                    }
                    "돌려주다" => {
                        self.end_of_statement()?;
                        if slots.len() != 1 {
                            return Err(Diag::syntax(msg::RETURN_NOT_ONE, info.span));
                        }
                        return Ok(Stmt::Return {
                            value: slots.pop().expect("값").expr,
                            span: info.span,
                        });
                    }
                    _ => {}
                }
            }
            match info.ending {
                Ending::AdnominalPast | Ending::AdnominalPres | Ending::Interrogative => {
                    let (value, kept) = self.reduce(slots, info)?;
                    slots = kept;
                    self.push(&mut slots, value)?;
                }
                Ending::Final | Ending::Conjunctive => {
                    let closing = info.ending == Ending::Final;
                    calls.push(CallExpr {
                        verb: info.name,
                        slots: std::mem::take(&mut slots),
                        negated: info.negated,
                        asks: false,
                        tail: None,
                        span: info.span,
                    });
                    if closing {
                        self.end_of_statement()?;
                        let span = calls[0].span;
                        return Ok(Stmt::Exec { calls, span });
                    }
                }
                Ending::Conditional => {
                    return Err(Diag::syntax(msg::EXEC_CONDITIONAL, info.span)
                        .with_hint(msg::EXEC_CONDITIONAL_HELP))
                }
                other => {
                    return Err(Diag::syntax(
                        msg::exec_bad_ending(ending_label(other)),
                        info.span,
                    ))
                }
            }
        }
    }

    fn range_loop(&mut self, slots: Vec<Slot>, span: Span) -> Result<Stmt> {
        let (mut start, mut stop, mut step, mut variable) = (None, None, None, None);
        for slot in slots {
            match slot.marker {
                Marker::Case("부터") => start = Some(slot.expr),
                Marker::Case("까지") => stop = Some(slot.expr),
                Marker::Step => step = Some(slot.expr),
                Marker::Case("마다") => {
                    let Some(name) = slot.expr.as_name() else {
                        return Err(Diag::syntax(msg::EACH_NOT_NAME, slot.expr.span()));
                    };
                    variable = Some(name.to_string());
                }
                _ => {}
            }
        }
        let Some(variable) = variable else {
            return Err(Diag::syntax(msg::LOOP_NO_EACH, span));
        };
        let (Some(start), Some(stop)) = (start, stop) else {
            return Err(Diag::syntax(msg::LOOP_NO_RANGE, span));
        };
        let body = self.block()?;
        Ok(Stmt::Loop {
            kind: LoopKind::Range {
                variable,
                start,
                stop,
                step,
            },
            body,
            span,
        })
    }
}

fn not_dictionary_form(span: Span) -> Diag {
    Diag::syntax(msg::HEAD_NOT_DICT, span).with_hint(msg::HEAD_NOT_DICT_HELP)
}
