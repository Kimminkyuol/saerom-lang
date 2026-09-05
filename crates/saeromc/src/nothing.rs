//! 없음일 수 있는 값을 검사 없이 쓰는 곳을 잡는다.

use crate::builtins::Builtin;
use crate::diag::{Diag, Span};
use crate::hir::*;
use crate::msg;
use crate::types::Types;
use std::collections::HashSet;

// 없음을 받아도 되는 내장. 나머지는 진짜 값이 있어야 한다.
fn tolerates(op: Builtin) -> bool {
    matches!(
        op,
        Builtin::Print | Builtin::Stop | Builtin::Equal | Builtin::Truthy | Builtin::Clone
    )
}

pub fn check(program: &Program, types: &Types) -> Vec<(ModuleId, Diag)> {
    let mut pass = Pass {
        program,
        types,
        errors: Vec::new(),
        module: 0,
        function: None,
    };
    for (id, module) in program.modules.iter().enumerate() {
        pass.module = id as ModuleId;
        pass.function = None;
        let mut sure = HashSet::new();
        pass.block(&module.init, &mut sure);
    }
    for (id, found) in program.functions.iter().enumerate() {
        pass.module = found.module;
        pass.function = Some(id as FuncId);
        let mut sure: HashSet<Place> = found.params.iter().map(|&s| Place::Local(s)).collect();
        pass.block(&found.body, &mut sure);
    }
    pass.errors
}

type Sure = HashSet<Place>;

struct Pass<'a> {
    program: &'a Program,
    types: &'a Types,
    errors: Vec<(ModuleId, Diag)>,
    module: ModuleId,
    function: Option<FuncId>,
}

fn place_of(expr: &Expr) -> Option<Place> {
    match expr {
        Expr::Local(slot) => Some(Place::Local(*slot)),
        Expr::Global(slot) => Some(Place::Global(*slot)),
        _ => None,
    }
}

// `X가 없음이면` / `X가 없음이 아니면` 을 알아본다.
fn tests_nothing(test: &Expr) -> Option<(Place, bool)> {
    match test {
        Expr::Not(inner) => tests_nothing(inner).map(|(place, yes)| (place, !yes)),
        Expr::Call {
            callee: Callee::Op(Builtin::Equal),
            args,
            ..
        } => {
            let [left, right] = args.as_slice() else {
                return None;
            };
            match (place_of(left), right, left, place_of(right)) {
                (Some(place), Expr::Nothing, _, _) => Some((place, true)),
                (_, _, Expr::Nothing, Some(place)) => Some((place, true)),
                _ => None,
            }
        }
        _ => None,
    }
}

// 블록이 아래로 이어지는가.
fn flows(body: &[Stmt]) -> bool {
    let Some(last) = body.last() else {
        return true;
    };
    match last {
        Stmt::Return { .. } | Stmt::Break | Stmt::Continue => false,
        Stmt::Eval(Expr::Call {
            callee: Callee::Op(Builtin::Stop),
            ..
        }) => false,
        Stmt::If {
            branches,
            otherwise: Some(otherwise),
        } => branches.iter().any(|(_, body)| flows(body)) || flows(otherwise),
        _ => true,
    }
}

fn spot(expr: &Expr) -> Option<Span> {
    match expr {
        Expr::Field { span, .. }
        | Expr::Index { span, .. }
        | Expr::Pick { span, .. }
        | Expr::Call { span, .. }
        | Expr::Ask { span, .. } => Some(*span),
        _ => None,
    }
}

impl Pass<'_> {
    fn blame(&mut self, span: Span, what: &str) {
        self.errors
            .push((
                self.module,
                Diag::new(msg::VALUE, msg::may_be_nothing(what), span)
            ));
    }

    // 진짜 값이 있어야 하는 자리
    fn demand(&mut self, expr: &Expr, sure: &Sure, span: Span, what: &str) {
        self.walk(expr, sure);
        if !self.types.of(self.function, expr).maybe_nothing() {
            return;
        }
        if place_of(expr).is_some_and(|place| sure.contains(&place)) {
            return;
        }
        self.blame(spot(expr).unwrap_or(span), what);
    }

    fn walk(&mut self, expr: &Expr, sure: &Sure) {
        match expr {
            Expr::Field { owner, span, .. } => self.demand(owner, sure, *span, msg::OWNER),
            Expr::Index { owner, place, span } => {
                self.demand(owner, sure, *span, msg::OWNER);
                self.demand(place, sure, *span, msg::PLACE);
            }
            Expr::Pick { owner, key, span } => {
                self.demand(owner, sure, *span, msg::OWNER);
                self.demand(key, sure, *span, msg::PLACE);
            }
            Expr::Call { callee, args, span } => {
                let strict = match callee {
                    Callee::User(id) => {
                        Some(self.program.functions[*id as usize].name.to_string())
                    }
                    Callee::Op(op) if tolerates(*op) => None,
                    Callee::Op(op) => Some(crate::builtins::named(*op).to_string()),
                };
                for arg in args {
                    match &strict {
                        Some(verb) => {
                            let what = msg::argument_of(verb);
                            self.demand(arg, sure, *span, &what)
                        }
                        None => self.walk(arg, sure),
                    }
                }
            }
            Expr::Template(items) => {
                for item in items {
                    self.walk(item, sure);
                }
            }
            Expr::Table(items, entries) => {
                for item in items {
                    self.walk(item, sure);
                }
                for (_, value) in entries {
                    self.walk(value, sure);
                }
            }
            Expr::Not(inner) | Expr::Ask { value: inner, .. } => self.walk(inner, sure),
            Expr::And(left, right) | Expr::Or(left, right) => {
                self.walk(left, sure);
                self.walk(right, sure);
            }
            _ => {}
        }
    }

    fn block(&mut self, body: &[Stmt], sure: &mut Sure) {
        for statement in body {
            self.stmt(statement, sure);
        }
    }

    fn stmt(&mut self, statement: &Stmt, sure: &mut Sure) {
        match statement {
            Stmt::Set { place, value } => {
                self.walk(value, sure);
                let safe = !self.types.of(self.function, value).maybe_nothing()
                    || place_of(value).is_some_and(|found| sure.contains(&found));
                if safe {
                    sure.insert(*place);
                } else {
                    sure.remove(place);
                }
            }
            Stmt::SetField {
                owner, value, span, ..
            } => {
                self.demand(owner, sure, *span, msg::OWNER);
                self.walk(value, sure);
            }
            Stmt::SetPick {
                owner, key, value, span
            } => {
                self.demand(owner, sure, *span, msg::OWNER);
                self.demand(key, sure, *span, msg::PLACE);
                self.walk(value, sure);
            }
            Stmt::SetAt {
                owner,
                place,
                value,
                span,
            } => {
                self.demand(owner, sure, *span, msg::OWNER);
                self.demand(place, sure, *span, msg::PLACE);
                self.walk(value, sure);
            }
            Stmt::Eval(value) => self.walk(value, sure),
            Stmt::Return { value, .. } => self.walk(value, sure),
            Stmt::Break | Stmt::Continue => {}
            Stmt::If {
                branches,
                otherwise,
            } => {
                let entry = sure.clone();
                let mut ends: Vec<Sure> = Vec::new();
                let mut carried = entry.clone();
                for (test, body) in branches {
                    self.walk(test, &carried);
                    let mut inner = carried.clone();
                    if let Some((place, asks_nothing)) = tests_nothing(test) {
                        if asks_nothing {
                            inner.remove(&place);
                            carried.insert(place);
                        } else {
                            inner.insert(place);
                            carried.remove(&place);
                        }
                    }
                    self.block(body, &mut inner);
                    // 되돌아오지 않는 가지는 뒤쪽에 아무 말도 하지 않는다.
                    if flows(body) {
                        ends.push(inner);
                    }
                }
                let mut inner = carried;
                if let Some(body) = otherwise {
                    self.block(body, &mut inner);
                    if flows(body) {
                        ends.push(inner);
                    }
                } else {
                    ends.push(inner);
                }
                let mut found = ends.pop().unwrap_or(entry);
                for other in &ends {
                    found.retain(|place| other.contains(place));
                }
                *sure = found;
            }
            Stmt::Range {
                place,
                start,
                stop,
                step,
                body,
                span,
            } => {
                self.demand(start, sure, *span, msg::RANGE_BOUND);
                self.demand(stop, sure, *span, msg::RANGE_BOUND);
                if let Some(step) = step {
                    self.demand(step, sure, *span, msg::RANGE_BOUND);
                }
                sure.insert(*place);
                let mut inner = sure.clone();
                self.block(body, &mut inner);
            }
            Stmt::Each {
                place, over, body, span,
            } => {
                self.demand(over, sure, *span, msg::OWNER);
                let mut inner = sure.clone();
                inner.insert(*place);
                self.block(body, &mut inner);
            }
            Stmt::While { test, body } => {
                self.walk(test, sure);
                let mut inner = sure.clone();
                self.block(body, &mut inner);
            }
        }
    }
}
