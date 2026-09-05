//! 확정 대입 검사.

use crate::ast::*;
use crate::diag::Diag;
use crate::msg;
use std::collections::HashSet;

type Live = HashSet<String>;

pub struct Check<'a> {
    locals: &'a Live,
    errors: Vec<Diag>,
    seen: Live,
}

pub fn check(body: &[Stmt], locals: &Live, params: &Live) -> Vec<Diag> {
    let mut pass = Check {
        locals,
        errors: Vec::new(),
        seen: Live::new(),
    };
    let mut live = params.clone();
    pass.block(body, &mut live);
    pass.errors
}

fn meet(branches: Vec<Live>, fallback: &Live) -> Live {
    let mut found = match branches.first() {
        Some(first) => first.clone(),
        None => return fallback.clone(),
    };
    for other in &branches[1..] {
        found.retain(|name| other.contains(name));
    }
    found
}

impl Check<'_> {
    fn block(&mut self, body: &[Stmt], live: &mut Live) -> bool {
        // 반환값: 아래로 이어지는가
        for statement in body {
            if !self.stmt(statement, live) {
                return false;
            }
        }
        true
    }

    fn stmt(&mut self, statement: &Stmt, live: &mut Live) -> bool {
        match statement {
            Stmt::Declare { assigns, .. } => {
                for (target, value) in assigns {
                    self.expr(value, live);
                    if target.fields.is_empty() {
                        live.insert(target.root.clone());
                        continue;
                    }
                    self.name(&target.root, target.span, live);
                    for field in &target.fields {
                        match field {
                            Selector::Pick(key) | Selector::Spot(key) => self.expr(key, live),
                            Selector::Name(_) => {}
                        }
                    }
                }
            }
            Stmt::Exec { calls, .. } => {
                for call in calls {
                    self.call(call, live);
                }
                // 종료한다는 되돌아오지 않는다
                if calls.iter().any(|call| call.verb == "종료하다") {
                    return false;
                }
            }
            Stmt::Value { expr, .. } => self.expr(expr, live),
            Stmt::Return { value, .. } => {
                self.expr(value, live);
                return false;
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => return false,
            Stmt::If {
                branches,
                otherwise,
                ..
            } => {
                let entry = live.clone();
                let mut ends: Vec<Live> = Vec::new();
                for (test, body) in branches {
                    let mut inner = entry.clone();
                    self.expr(test, &mut inner);
                    if self.block(body, &mut inner) {
                        ends.push(inner);
                    }
                }
                match otherwise {
                    Some(body) => {
                        let mut inner = entry.clone();
                        if self.block(body, &mut inner) {
                            ends.push(inner);
                        }
                        if ends.is_empty() {
                            return false;
                        }
                        *live = meet(ends, &entry);
                    }
                    None => {
                        ends.push(entry.clone());
                        *live = meet(ends, &entry);
                    }
                }
            }
            Stmt::Loop { kind, body, .. } => {
                let mut inner = live.clone();
                match kind {
                    LoopKind::Range {
                        variable,
                        start,
                        stop,
                        step,
                    } => {
                        self.expr(start, live);
                        self.expr(stop, live);
                        if let Some(step) = step {
                            self.expr(step, live);
                        }
                        // 빈 범위여도 시작값은 들어간다
                        live.insert(variable.clone());
                        inner = live.clone();
                    }
                    LoopKind::Each { variable, over } => {
                        self.expr(over, live);
                        inner = live.clone();
                        inner.insert(variable.clone());
                    }
                    LoopKind::While { test } => self.expr(test, live),
                }
                // 안 돌 수 있으니 안에서 매긴 건 못 나온다
                self.block(body, &mut inner);
            }
            Stmt::Define { .. } | Stmt::Noun { .. } | Stmt::Import { .. } => {}
        }
        true
    }

    fn call(&mut self, call: &CallExpr, live: &mut Live) {
        for slot in &call.slots {
            self.expr(&slot.expr, live);
        }
    }

    fn name(&mut self, name: &str, span: crate::diag::Span, live: &Live) {
        if !self.locals.contains(name) || live.contains(name) {
            return;
        }
        if !self.seen.insert(name.to_string()) {
            return;
        }
        self.errors
            .push(Diag::name(msg::not_assigned(name), span));
    }

    fn expr(&mut self, expr: &Expr, live: &mut Live) {
        match expr {
            Expr::Literal { .. } => {}
            Expr::Name { name, span } => self.name(name, *span, live),
            Expr::Table { items, entries, .. } => {
                for item in items {
                    self.expr(item, live);
                }
                for (_, value) in entries {
                    self.expr(value, live);
                }
            }
            Expr::Template { parts, .. } => {
                for part in parts {
                    if let TemplatePart::Expr(inner) = part {
                        self.expr(inner, live);
                    }
                }
            }
            Expr::Field { owner, name, span } => {
                self.expr(owner, live);
                // 자리번째
                if let Some(head) = name.strip_suffix("번째") {
                    if head.parse::<i64>().is_err() {
                        self.name(head, *span, live);
                    }
                }
            }
            Expr::Pick { owner, key, .. } => {
                self.expr(owner, live);
                self.expr(key, live);
            }
            Expr::Spot { owner, place, .. } => {
                self.expr(owner, live);
                self.expr(place, live);
            }
            Expr::Call(call) => self.call(call, live),
            Expr::Passive(passive) => {
                self.expr(&passive.head, live);
                for slot in &passive.slots {
                    self.expr(&slot.expr, live);
                }
            }
            Expr::And { left, right, .. } | Expr::Or { left, right, .. } => {
                self.expr(left, live);
                self.expr(right, live);
            }
        }
    }
}
