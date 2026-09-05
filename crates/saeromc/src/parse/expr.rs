//! 값과 구절.

use super::*;

impl<'a> Parser<'a> {
    pub(super) fn take_verb(&mut self) -> Result<VerbInfo> {
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

    pub(super) fn starts_predicate(&self, offset: usize) -> bool {
        let Tok::Name(name) = &self.ahead(offset).tok else {
            return false;
        };
        matches!(self.ahead(offset + 1).tok, Tok::Copula { .. })
            && self.program.signatures.knows(&format!("{name}이다"))
    }

    pub(super) fn chain(&mut self, value: Expr) -> Result<Expr> {
        let mut value = value;
        loop {
            if !matches!(self.peek(), Tok::Particle { canon: "의", .. }) {
                return Ok(value);
            }
            let span = self.ahead(1).span;
            match &self.ahead(1).tok {
                Tok::Name(field) => {
                    if self.program.modules.contains(field) {
                        return Ok(value);
                    }
                    if let Tok::Copula { ending } = self.ahead(2).tok {
                        if ending != Ending::Final {
                            return Ok(value);
                        }
                    }
                    let name = field.clone();
                    self.at += 2;
                    value = Expr::Field {
                        owner: Box::new(value),
                        name,
                        span,
                    };
                }
                Tok::Str(text) => {
                    let name = text.clone();
                    self.at += 2;
                    value = Expr::Field {
                        owner: Box::new(value),
                        name,
                        span,
                    };
                }
                Tok::Symbol('(') => {
                    self.at += 1;
                    let key = self.selector()?;
                    value = self.wrap_selector(value, key, span);
                }
                _ => return Ok(value),
            }
        }
    }

    pub(super) fn selector(&mut self) -> Result<Expr> {
        let span = self.span();
        self.at += 1;
        let key = self.grouped(span)?;
        self.expect(&Tok::Symbol(')'), msg::WANT_CLOSE)?;
        Ok(key)
    }

    pub(super) fn wrap_selector(&mut self, owner: Expr, key: Expr, span: Span) -> Expr {
        if matches!(self.peek(), Tok::Name(name) if name == "번째") {
            self.at += 1;
            return Expr::Spot {
                owner: Box::new(owner),
                place: Box::new(key),
                span,
            };
        }
        Expr::Pick {
            owner: Box::new(owner),
            key: Box::new(key),
            span,
        }
    }

    pub(super) fn push(&mut self, slots: &mut Vec<Slot>, first: Expr) -> Result<()> {
        let mut items = vec![self.chain(first)?];
        while matches!(self.peek(), Tok::Particle { role: "conj", .. })
            && starts_value(&self.ahead(1).tok)
            && !self.starts_predicate(1)
        {
            self.at += 1;
            let next = self.primary()?;
            items.push(self.chain(next)?);
        }
        let value = if items.len() == 1 {
            items.pop().expect("항 하나")
        } else {
            let span = items[0].span();
            Expr::Table {
                items,
                entries: Vec::new(),
                span,
            }
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

    // 문장 끝 용언은 남은 자리를 다 가져간다. 조사가 안 맞으면 앞에서 잘못
    // 묶은 것이므로 되짚기에게 알린다.
    pub(super) fn verify(&mut self, verb: &str, slots: &[Slot]) {
        let ways = self.program.signatures.ways(verb);
        if ways.is_empty() || verb == "이다" {
            return;
        }
        let used: Vec<Marker> = slots
            .iter()
            .map(|slot| slot.marker)
            .filter(|marker| *marker != Marker::Module)
            .collect();
        if !ways.iter().any(|way| crate::sig::same(&used, way)) {
            self.stuck = true;
        }
    }

    pub(super) fn split_slots(&mut self, verb: &str, slots: Vec<Slot>) -> (Vec<Slot>, Vec<Slot>) {
        let ways = self.program.signatures.ways(verb);
        let (structural, arguments): (Vec<Slot>, Vec<Slot>) = slots
            .into_iter()
            .partition(|slot| !slot.marker.is_argument());
        let fixed: Vec<Marker> = structural
            .iter()
            .map(|slot| slot.marker)
            .filter(|marker| *marker != Marker::Module)
            .collect();
        let total = arguments.len();
        // 조사가 딱 맞는 꼬리만 후보다. 긴 것부터 담는다.
        // 시그니처를 모르는 용언(동사 자리 매개변수 등)은 모든 길이가 후보다.
        let blind = ways.is_empty();
        let mut candidates: Vec<usize> = Vec::new();
        for count in (0..=total).rev() {
            let mut used = fixed.clone();
            used.extend(arguments[total - count..].iter().map(|slot| slot.marker));
            if blind || ways.iter().any(|way| crate::sig::same(&used, way)) {
                candidates.push(count);
            }
        }
        let index = self.picks.len();
        self.picks.push(candidates.len().max(1));
        let pick = self.plan.get(index).copied().unwrap_or(0);
        let Some(&count) = candidates.get(pick).or_else(|| candidates.first()) else {
            // 딱 맞는 게 없다. 남은 인자를 다 준다 — 그래야 resolve 가 이 용언을
            // 짚어 조사 오류를 낸다. 자리를 남기면 엉뚱한 데서 터진다.
            self.stuck = true;
            let mut taken = structural;
            taken.extend(arguments);
            return (Vec::new(), taken);
        };
        let mut taken = structural;
        taken.extend_from_slice(&arguments[total - count..]);
        (arguments[..total - count].to_vec(), taken)
    }

    pub(super) fn reduce(&mut self, slots: Vec<Slot>, info: VerbInfo) -> Result<(Expr, Vec<Slot>)> {
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

    pub(super) fn primary(&mut self) -> Result<Expr> {
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
            Tok::Keyword(word) if word == "묶음" => {
                self.at += 1;
                Ok(Expr::Table {
                    items: Vec::new(),
                    entries: Vec::new(),
                    span,
                })
            }
            Tok::Name(name) => {
                let name = name.clone();
                self.at += 1;
                Ok(Expr::Name { name, span })
            }
            Tok::Symbol('(') => {
                self.at += 1;
                let value = self.grouped(span)?;
                self.expect(&Tok::Symbol(')'), msg::WANT_CLOSE)?;
                Ok(value)
            }
            // 조사와 동음인 낱말(가·는·의·로…)은 이름이 될 수 없다. 그냥 "값이 아님"
            // 으로 흘리면 원인을 알 수 없어 따로 짚는다.
            Tok::Particle { .. } => Err(Diag::syntax(
                msg::not_a_value(&describe(&token.tok)),
                span,
            )
            .with_hint(msg::NAME_IS_PARTICLE)),
            other => Err(Diag::syntax(msg::not_a_value(&describe(other)), span)),
        }
    }

    pub(super) fn reduce_until(&mut self, stop: fn(&Tok) -> bool, what: &str) -> Result<Expr> {
        let mut slots: Vec<Slot> = Vec::new();
        loop {
            let token = self.ahead(0);
            if stop(&token.tok) || matches!(token.tok, Tok::Eof) {
                if slots.len() != 1 {
                    return Err(Diag::syntax(msg::not_one(what), token.span));
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

    pub(super) fn fragment(&mut self, source: &str, at: Span) -> Result<Expr> {
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
            inside: false,
            plan: Vec::new(),
            picks: Vec::new(),
            stuck: false,
        };
        inner.replanned(|found| {
            // 괄호 없이도 `3의 묶음` 을 받는다.
            if found.closes_table_at(|tok| matches!(tok, Tok::Newline | Tok::Eof)) {
                let span = found.span();
                return found.table_body(span);
            }
            found.reduce_until(
                |tok| matches!(tok, Tok::Newline | Tok::Eof),
                msg::WANT_EMBEDDED,
            )
        })
    }
}
