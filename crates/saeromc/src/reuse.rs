use crate::builtins::Builtin;
use crate::hir::*;
use crate::types::{Ty, Types};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Global(GlobalId),
    Local(FuncId, LocalId),
}

#[derive(Default)]
pub struct Reuse {
    kept: HashSet<Key>,
}

impl Reuse {
    // 제자리 잇기를 해도 되는 자리 (문자열 누적)
    pub fn allows(&self, function: Option<FuncId>, place: Place) -> bool {
        key_of(function, place).is_some_and(|key| self.kept.contains(&key))
    }
}

fn key_of(function: Option<FuncId>, place: Place) -> Option<Key> {
    match place {
        Place::Global(slot) => Some(Key::Global(slot)),
        Place::Local(slot) => function.map(|id| Key::Local(id, slot)),
    }
}

fn place_of(expr: &Expr) -> Option<Place> {
    match expr {
        Expr::Local(slot) => Some(Place::Local(*slot)),
        Expr::Global(slot) => Some(Place::Global(*slot)),
        _ => None,
    }
}

#[derive(Default)]
struct Scan {
    gone: HashSet<Key>,
    stale: HashSet<Key>,
    // 순회 대상. 몸통 안에서 덮어쓰면 돌고 있는 것을 놓아주게 된다.
    walked: HashSet<Key>,
    seen: HashMap<Key, bool>,
}

pub fn find(program: &Program, types: &Types) -> Reuse {
    let mut scan = Scan::default();
    for module in &program.modules {
        scan.block(None, &module.init);
    }
    for (id, function) in program.functions.iter().enumerate() {
        let id = id as FuncId;
        for &slot in &function.params {
            scan.stale.insert(Key::Local(id, slot));
        }
        scan.block(Some(id), &function.body);
    }
    let alone = |key: Key, holds: bool| {
        holds
            && !scan.gone.contains(&key)
            && !scan.stale.contains(&key)
            && !scan.walked.contains(&key)
    };
    let kept = scan
        .seen
        .iter()
        .filter(|&(&key, &holds)| alone(key, holds) && ty_of(types, key) == Ty::Str)
        .map(|(&key, _)| key)
        .collect();
    Reuse { kept }
}

fn ty_of(types: &Types, key: Key) -> Ty {
    let (function, place) = match key {
        Key::Global(slot) => (None, Place::Global(slot)),
        Key::Local(id, slot) => (Some(id), Place::Local(slot)),
    };
    types.place(function, place)
}

// 새 값을 만드는가. 복사하다는 뺀다 — 문자열이면 같은 값을 그대로 돌려준다.
fn makes_fresh(expr: &Expr) -> bool {
    match expr {
        Expr::Str(_) | Expr::Template(_) | Expr::Table(..) => true,
        Expr::Call {
            callee: Callee::Op(op),
            ..
        } => matches!(op, Builtin::Add | Builtin::Convert | Builtin::Read),
        _ => false,
    }
}

fn holds_value(op: Builtin) -> bool {
    matches!(op, Builtin::Clone | Builtin::Push)
}

impl Scan {
    fn block(&mut self, function: Option<FuncId>, body: &[Stmt]) {
        for statement in body {
            self.stmt(function, statement);
        }
    }

    fn stmt(&mut self, function: Option<FuncId>, statement: &Stmt) {
        match statement {
            Stmt::Set { place, value } => {
                if let Some(key) = key_of(function, *place) {
                    let fresh = makes_fresh(value);
                    let entry = self.seen.entry(key).or_insert(true);
                    *entry &= fresh;
                }
                self.expr(function, value, true);
            }
            Stmt::SetField { owner, value, .. } => {
                self.expr(function, owner, false);
                self.expr(function, value, true);
            }
            Stmt::SetAt {
                owner,
                place,
                value,
                ..
            } => {
                self.expr(function, owner, false);
                self.expr(function, place, false);
                self.expr(function, value, true);
            }
            Stmt::Each {
                place, over, body, ..
            } => {
                if let Some(key) = key_of(function, *place) {
                    self.stale.insert(key);
                }
                if let Some(key) = place_of(over).and_then(|found| key_of(function, found)) {
                    self.walked.insert(key);
                }
                self.expr(function, over, false);
                self.block(function, body);
            }
            Stmt::SetPick {
                owner, key, value, ..
            } => {
                self.expr(function, owner, false);
                self.expr(function, key, true);
                self.expr(function, value, true);
            }
            Stmt::Eval(value) => self.expr(function, value, false),
            Stmt::If {
                branches,
                otherwise,
            } => {
                for (test, body) in branches {
                    self.expr(function, test, false);
                    self.block(function, body);
                }
                if let Some(body) = otherwise {
                    self.block(function, body);
                }
            }
            Stmt::Range {
                place,
                start,
                stop,
                step,
                body,
                ..
            } => {
                if let Some(key) = key_of(function, *place) {
                    self.stale.insert(key);
                }
                self.expr(function, start, false);
                self.expr(function, stop, false);
                if let Some(step) = step {
                    self.expr(function, step, false);
                }
                self.block(function, body);
            }
            Stmt::While { test, body } => {
                self.expr(function, test, false);
                self.block(function, body);
            }
            Stmt::Break | Stmt::Continue => {}
            Stmt::Return { value, .. } => self.expr(function, value, true),
        }
    }

    fn expr(&mut self, function: Option<FuncId>, expr: &Expr, escapes: bool) {
        match expr {
            Expr::Local(_) | Expr::Global(_) => {
                if escapes {
                    if let Some(key) = place_of(expr).and_then(|place| key_of(function, place))
                    {
                        self.gone.insert(key);
                    }
                }
            }
            Expr::Table(items, entries) => {
                for item in items {
                    self.expr(function, item, true);
                }
                for (_, item) in entries {
                    self.expr(function, item, true);
                }
            }
            Expr::Template(parts) => {
                for part in parts {
                    self.expr(function, part, false);
                }
            }
            Expr::Field { owner, .. } => self.expr(function, owner, false),
            Expr::Index { owner, place, .. } => {
                self.expr(function, owner, false);
                self.expr(function, place, false);
            }
            Expr::Pick { owner, key, .. } => {
                self.expr(function, owner, false);
                self.expr(function, key, true);
            }
            Expr::Call { callee, args, .. } => {
                let holds = match callee {
                    Callee::User(_) => true,
                    Callee::Op(op) => holds_value(*op),
                };
                for arg in args {
                    self.expr(function, arg, holds);
                }
            }
            Expr::Not(inner) | Expr::Ask { value: inner, .. } => {
                self.expr(function, inner, false)
            }
            Expr::And(left, right) | Expr::Or(left, right) => {
                self.expr(function, left, false);
                self.expr(function, right, false);
            }
            _ => {}
        }
    }
}
