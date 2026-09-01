use crate::ast::{Call, Expr, Slot, Stmt};
use crate::diag::{Diag, Result, Span};
use crate::lex::{Tok, Token};

pub fn parse(tokens: &[Token]) -> Result<Vec<Stmt>> {
    Parser { tokens, at: 0 }.program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &'a Token {
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &'a Token {
        let token = self.peek();
        self.at += 1;
        token
    }

    fn eat(&mut self, wanted: &Tok) -> bool {
        if &self.peek().tok == wanted {
            self.at += 1;
            return true;
        }
        false
    }

    fn expect(&mut self, wanted: &Tok, what: &str) -> Result<()> {
        if self.eat(wanted) {
            return Ok(());
        }
        Err(Diag::syntax(format!("{what} 없음"), self.peek().span))
    }

    fn program(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        loop {
            while self.eat(&Tok::Newline) {}
            if self.peek().tok == Tok::Eof {
                return Ok(statements);
            }
            statements.push(self.statement()?);
        }
    }

    fn statement(&mut self) -> Result<Stmt> {
        let call = self.call()?;
        self.expect(&Tok::Symbol('.'), "문장을 닫는 마침표가")?;
        self.expect(&Tok::Newline, "줄바꿈이")?;
        Ok(Stmt::Exec(vec![call]))
    }

    fn call(&mut self) -> Result<Call> {
        let start = self.peek().span;
        let mut slots = Vec::new();
        loop {
            match &self.peek().tok {
                Tok::Verb { name, ending, .. } => {
                    let (name, ending) = (name.clone(), *ending);
                    let span = self.advance().span;
                    if ending != "final" {
                        return Err(Diag::syntax(
                            format!("'{name}'의 어미 '{ending}'는 아직 지원하지 않음"),
                            span,
                        ));
                    }
                    return Ok(Call {
                        verb: name,
                        slots,
                        span: cover(start, span),
                    });
                }
                Tok::Eof | Tok::Newline => {
                    return Err(Diag::syntax("문장에 동사가 없음", self.peek().span))
                }
                _ => slots.push(self.slot()?),
            }
        }
    }

    fn slot(&mut self) -> Result<Slot> {
        let token = self.advance();
        let expr = match &token.tok {
            Tok::Str(text) => Expr::Str(text.clone()),
            Tok::Number(value) => Expr::Number(value.clone()),
            Tok::Name(name) => Expr::Name(name.clone()),
            other => return Err(Diag::syntax(format!("식이 올 자리: {other:?}"), token.span)),
        };
        let particle = match self.peek().tok {
            Tok::Particle { canon, .. } => {
                self.at += 1;
                Some(canon)
            }
            _ => None,
        };
        let span = cover(token.span, self.tokens[self.at - 1].span);
        Ok(Slot {
            particle,
            expr,
            span,
        })
    }
}

fn cover(start: Span, end: Span) -> Span {
    if start.line == end.line {
        Span::new(start.line, start.col, end.end)
    } else {
        start
    }
}
