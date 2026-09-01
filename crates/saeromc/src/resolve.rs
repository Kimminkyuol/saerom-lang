use crate::ast::{self, DefKind, LoopKind, Stmt as ASt};
use crate::builtins;
use crate::diag::{suggest, Diag, Span};
use crate::hir::*;
use crate::intern::Interner;
use crate::load::{Loaded, UnitId};
use crate::sig::Marker;
use std::collections::HashMap;
use std::rc::Rc;

const TYPE_VALUES: [&str; 4] = ["정수", "실수", "문자열", "논리값"];

struct Verb {
    name: String,
    params: Vec<Marker>,
    func: FuncId,
}

#[derive(Default)]
struct Tables {
    globals: HashMap<String, GlobalId>,
    verbs: Vec<Verb>,
    nouns: HashMap<String, FuncId>,
    modules: HashMap<String, UnitId>,
}

struct Frame {
    unit: UnitId,
    locals: HashMap<String, LocalId>,
    count: u32,
    inside_function: bool,
}

impl Frame {
    fn place(&mut self, name: &str, tables: &mut Tables, globals: &mut u32) -> Place {
        if self.inside_function {
            if let Some(&found) = self.locals.get(name) {
                return Place::Local(found);
            }
            let slot = self.count;
            self.count += 1;
            self.locals.insert(name.to_string(), slot);
            return Place::Local(slot);
        }
        Place::Global(global_slot(name, tables, globals))
    }
}

fn global_slot(name: &str, tables: &mut Tables, globals: &mut u32) -> GlobalId {
    if let Some(&found) = tables.globals.get(name) {
        return found;
    }
    let slot = *globals;
    *globals += 1;
    tables.globals.insert(name.to_string(), slot);
    slot
}

pub struct Resolver<'a> {
    loaded: &'a Loaded,
    tables: Vec<Tables>,
    functions: Vec<Function>,
    bodies: Vec<&'a ASt>,
    owners: Vec<UnitId>,
    names: Interner,
    globals: u32,
    module_of: Vec<ModuleId>,
    errors: Vec<Diag>,
}

pub fn resolve(loaded: &Loaded) -> Result<Program, Vec<Diag>> {
    let mut resolver = Resolver {
        loaded,
        tables: (0..loaded.units.len()).map(|_| Tables::default()).collect(),
        functions: Vec::new(),
        bodies: Vec::new(),
        owners: Vec::new(),
        names: Interner::default(),
        globals: 0,
        module_of: Vec::new(),
        errors: Vec::new(),
    };
    let order = order_of(loaded);
    let mut module_of = vec![0 as ModuleId; loaded.units.len()];
    for (index, &unit) in order.iter().enumerate() {
        module_of[unit] = index as ModuleId;
    }
    resolver.module_of = module_of;
    for &unit in &order {
        resolver.declare(unit);
    }
    let modules = order
        .iter()
        .map(|&unit| resolver.lower_module(unit))
        .collect();
    let functions = resolver.lower_functions();
    if !resolver.errors.is_empty() {
        return Err(resolver.errors);
    }
    Ok(Program {
        modules,
        functions,
        names: resolver.names,
        globals: resolver.globals,
        order: (0..order.len() as ModuleId).collect(),
        root: resolver.module_of[loaded.root],
    })
}

fn order_of(loaded: &Loaded) -> Vec<UnitId> {
    let mut order = Vec::new();
    let mut seen = vec![false; loaded.units.len()];
    fn walk(loaded: &Loaded, unit: UnitId, seen: &mut [bool], order: &mut Vec<UnitId>) {
        if std::mem::replace(&mut seen[unit], true) {
            return;
        }
        for statement in &loaded.units[unit].statements {
            if let ASt::Import { path, .. } = statement {
                if let Some(next) = loaded.unit_of(path) {
                    walk(loaded, next, seen, order);
                }
            }
        }
        order.push(unit);
    }
    walk(loaded, loaded.root, &mut seen, &mut order);
    order
}

fn same_slots(left: &[Marker], right: &[Marker]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let (mut a, mut b) = (left.to_vec(), right.to_vec());
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

fn order_args(mut args: Vec<(Marker, Expr)>, params: &[Marker]) -> Vec<Expr> {
    let mut out = Vec::with_capacity(params.len());
    for marker in params {
        match args.iter().position(|(found, _)| found == marker) {
            Some(index) => out.push(args.remove(index).1),
            None => out.push(Expr::Nothing),
        }
    }
    out
}

fn shown(markers: &[Marker]) -> String {
    let used: Vec<&str> = markers
        .iter()
        .filter(|m| **m != Marker::Bare)
        .map(|m| m.label())
        .collect();
    if used.is_empty() {
        "없음".to_string()
    } else {
        used.join(", ")
    }
}

fn describe(verb: &str, params: &[Marker]) -> String {
    let mut out = String::new();
    for marker in params {
        if *marker != Marker::Bare {
            out.push_str(&format!("~{} ", marker.label()));
        }
    }
    out.push_str(verb);
    out
}

impl<'a> Resolver<'a> {
    fn note(&mut self, unit: UnitId, mut error: Diag) {
        error.unit = Some(unit);
        self.errors.push(error);
    }

    fn declare(&mut self, unit: UnitId) {
        let loaded = self.loaded;
        for name in TYPE_VALUES {
            global_slot(name, &mut self.tables[unit], &mut self.globals);
        }
        let statements = &loaded.units[unit].statements;
        for statement in statements {
            if let ASt::Import {
                module,
                names,
                path,
                span,
            } = statement
            {
                self.import(unit, module, names.as_deref(), path, *span);
            }
        }
        self.declare_functions(unit, statements);
        let mut bound = Vec::new();
        bind_names(statements, &mut bound);
        for name in bound {
            global_slot(&name, &mut self.tables[unit], &mut self.globals);
        }
    }

    fn import(
        &mut self,
        unit: UnitId,
        module: &str,
        names: Option<&[String]>,
        path: &std::path::Path,
        span: Span,
    ) {
        let Some(target) = self.loaded.unit_of(path) else {
            return;
        };
        let Some(names) = names else {
            self.tables[unit].modules.insert(module.to_string(), target);
            return;
        };
        for name in names {
            let verbs: Vec<(String, Vec<Marker>, FuncId)> = self.tables[target]
                .verbs
                .iter()
                .filter(|verb| &verb.name == name)
                .map(|verb| (verb.name.clone(), verb.params.clone(), verb.func))
                .collect();
            let noun = self.tables[target].nouns.get(name).copied();
            let global = self.tables[target].globals.get(name).copied();
            let taken = !verbs.is_empty() || noun.is_some() || global.is_some();
            for (name, params, func) in verbs {
                self.tables[unit].verbs.push(Verb { name, params, func });
            }
            if let Some(func) = noun {
                self.tables[unit].nouns.insert(name.clone(), func);
            }
            if let Some(slot) = global {
                self.tables[unit].globals.insert(name.clone(), slot);
            }
            if !taken {
                let known: Vec<&str> = self.tables[target]
                    .verbs
                    .iter()
                    .map(|verb| verb.name.as_str())
                    .chain(self.tables[target].nouns.keys().map(String::as_str))
                    .chain(self.tables[target].globals.keys().map(String::as_str))
                    .collect();
                let close = suggest(name, known).map(str::to_string);
                let mut error = Diag::name(format!("모듈 '{module}'에 '{name}' 없음"), span);
                if let Some(close) = close {
                    error = error.with_hint(format!("비슷한 이름: '{close}'"));
                }
                self.note(unit, error);
            }
        }
    }

    fn declare_functions(&mut self, unit: UnitId, statements: &'a [ASt]) {
        for statement in statements {
            match statement {
                ASt::Define {
                    name,
                    kind,
                    params,
                    span,
                    ..
                } => {
                    let func = self.functions.len() as FuncId;
                    self.functions.push(Function {
                        name: Rc::from(name.as_str()),
                        kind: match kind {
                            DefKind::Verb => Kind::Verb,
                            DefKind::Predicate => Kind::Predicate,
                        },
                        module: self.module_of[unit],
                        params: Vec::new(),
                        locals: 0,
                        body: Vec::new(),
                        span: *span,
                    });
                    self.bodies.push(statement);
                    self.owners.push(unit);
                    self.tables[unit].verbs.push(Verb {
                        name: name.clone(),
                        params: params.iter().map(|(marker, _)| *marker).collect(),
                        func,
                    });
                }
                ASt::Noun { name, span, .. } => {
                    let func = self.functions.len() as FuncId;
                    self.functions.push(Function {
                        name: Rc::from(name.as_str()),
                        kind: Kind::Noun,
                        module: self.module_of[unit],
                        params: Vec::new(),
                        locals: 0,
                        body: Vec::new(),
                        span: *span,
                    });
                    self.bodies.push(statement);
                    self.owners.push(unit);
                    self.tables[unit].nouns.insert(name.clone(), func);
                }
                _ => continue,
            }
        }
        for statement in statements {
            for block in blocks_of(statement) {
                self.declare_functions(unit, block);
            }
        }
    }
}

fn blocks_of(statement: &ASt) -> Vec<&[ASt]> {
    match statement {
        ASt::If {
            branches,
            otherwise,
            ..
        } => {
            let mut found: Vec<&[ASt]> =
                branches.iter().map(|(_, body)| body.as_slice()).collect();
            if let Some(body) = otherwise {
                found.push(body);
            }
            found
        }
        ASt::Loop { body, .. } | ASt::With { body, .. } => vec![body],
        _ => Vec::new(),
    }
}

fn bind_names(statements: &[ASt], into: &mut Vec<String>) {
    for statement in statements {
        match statement {
            ASt::Declare { target, .. } if target.fields.is_empty() => {
                into.push(target.root.clone())
            }
            ASt::Loop {
                kind: LoopKind::Range { variable, .. },
                ..
            } => into.push(variable.clone()),
            ASt::With { name, .. } => into.push(name.clone()),
            ASt::Define { .. } | ASt::Noun { .. } => continue,
            _ => {}
        }
        for block in blocks_of(statement) {
            bind_names(block, into);
        }
    }
}

impl<'a> Resolver<'a> {
    fn local(&mut self, frame: &mut Frame, name: &str) -> LocalId {
        match frame.place(name, &mut self.tables[frame.unit], &mut self.globals) {
            Place::Local(slot) => slot,
            Place::Global(_) => 0,
        }
    }

    fn lower_module(&mut self, unit: UnitId) -> Module {
        let loaded = self.loaded;
        let statements: &'a [ASt] = &loaded.units[unit].statements;
        let mut frame = Frame {
            unit,
            locals: HashMap::new(),
            count: 0,
            inside_function: false,
        };
        let mut init = Vec::new();
        for name in TYPE_VALUES {
            let slot = self.tables[unit].globals[name];
            init.push(Stmt::Set {
                place: Place::Global(slot),
                value: Expr::Str(Rc::from(name)),
            });
        }
        self.lower_into(&mut frame, statements, &mut init);
        let pairs: Vec<(String, FuncId)> = self.tables[unit]
            .nouns
            .iter()
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        let nouns = pairs
            .into_iter()
            .map(|(name, func)| (self.names.intern(&name), func))
            .collect();
        let found = &loaded.units[unit];
        let path = found
            .path
            .as_deref()
            .map_or_else(|| found.name.clone(), |path| path.display().to_string());
        Module {
            name: Rc::from(found.name.as_str()),
            path: Rc::from(path.as_str()),
            source: Rc::from(found.source.as_str()),
            init,
            nouns,
        }
    }

    fn lower_functions(&mut self) -> Vec<Function> {
        for index in 0..self.bodies.len() {
            let statement = self.bodies[index];
            let unit = self.owners[index];
            let mut frame = Frame {
                unit,
                locals: HashMap::new(),
                count: 0,
                inside_function: true,
            };
            let (params, body) = match statement {
                ASt::Define { params, body, .. } => {
                    let slots = params
                        .iter()
                        .map(|(_, name)| self.local(&mut frame, name))
                        .collect();
                    (slots, body)
                }
                ASt::Noun { owner, body, .. } => (vec![self.local(&mut frame, owner)], body),
                _ => continue,
            };
            let mut bound = Vec::new();
            bind_names(body, &mut bound);
            for name in bound {
                self.local(&mut frame, &name);
            }
            let lowered = self.lower_block(&mut frame, body);
            self.functions[index].params = params;
            self.functions[index].locals = frame.count;
            self.functions[index].body = lowered;
        }
        std::mem::take(&mut self.functions)
    }

    fn lower_block(&mut self, frame: &mut Frame, statements: &'a [ASt]) -> Vec<Stmt> {
        let mut out = Vec::new();
        self.lower_into(frame, statements, &mut out);
        out
    }

    fn lower_into(&mut self, frame: &mut Frame, statements: &'a [ASt], out: &mut Vec<Stmt>) {
        for statement in statements {
            self.lower_stmt(frame, statement, out);
        }
    }

    fn lower_stmt(&mut self, frame: &mut Frame, statement: &'a ASt, out: &mut Vec<Stmt>) {
        match statement {
            ASt::Declare {
                target,
                value,
                span,
            } => {
                let value = self.lower_expr(frame, value);
                if target.fields.is_empty() {
                    let place = frame.place(
                        &target.root,
                        &mut self.tables[frame.unit],
                        &mut self.globals,
                    );
                    out.push(Stmt::Set { place, value });
                    return;
                }
                let mut owner = self.read_name(frame, &target.root, target.span);
                for field in &target.fields[..target.fields.len() - 1] {
                    let field = self.names.intern(field);
                    owner = Expr::Field {
                        owner: Box::new(owner),
                        field,
                        span: target.span,
                    };
                }
                let field = self.names.intern(target.fields.last().expect("필드"));
                out.push(Stmt::SetField {
                    owner,
                    field,
                    value,
                    span: *span,
                });
            }
            ASt::Exec { calls, .. } => {
                for call in calls {
                    let value = self.lower_call(frame, call);
                    out.push(Stmt::Eval(value));
                }
            }
            ASt::Value { expr, .. } => {
                let value = self.lower_expr(frame, expr);
                out.push(Stmt::Eval(value));
            }
            ASt::If {
                branches,
                otherwise,
                ..
            } => {
                let branches = branches
                    .iter()
                    .map(|(test, body)| {
                        let test = self.lower_expr(frame, test);
                        (test, self.lower_block(frame, body))
                    })
                    .collect();
                let otherwise = otherwise.as_ref().map(|body| self.lower_block(frame, body));
                out.push(Stmt::If {
                    branches,
                    otherwise,
                });
            }
            ASt::Loop { kind, body, span } => match kind {
                LoopKind::Range {
                    variable,
                    start,
                    stop,
                    step,
                } => {
                    let start = self.lower_expr(frame, start);
                    let stop = self.lower_expr(frame, stop);
                    let step = step.as_ref().map(|step| self.lower_expr(frame, step));
                    let place =
                        frame.place(variable, &mut self.tables[frame.unit], &mut self.globals);
                    let body = self.lower_block(frame, body);
                    out.push(Stmt::Range {
                        place,
                        start,
                        stop,
                        step,
                        body,
                        span: *span,
                    });
                }
                LoopKind::While { test } => {
                    let test = self.lower_expr(frame, test);
                    let body = self.lower_block(frame, body);
                    out.push(Stmt::While { test, body });
                }
            },
            ASt::Break { .. } => out.push(Stmt::Break),
            ASt::Continue { .. } => out.push(Stmt::Continue),
            ASt::Return { value, span } => {
                let value = self.lower_expr(frame, value);
                out.push(Stmt::Return { value, span: *span });
            }
            ASt::With {
                call,
                name,
                body,
                span,
            } => {
                let call = self.lower_call(frame, call);
                let place = frame.place(name, &mut self.tables[frame.unit], &mut self.globals);
                let body = self.lower_block(frame, body);
                out.push(Stmt::With {
                    call,
                    place,
                    body,
                    span: *span,
                });
            }
            ASt::Define { .. } | ASt::Noun { .. } | ASt::Import { .. } => {}
        }
    }

    fn read_name(&mut self, frame: &mut Frame, name: &str, span: Span) -> Expr {
        if let Some(&slot) = frame.locals.get(name) {
            return Expr::Local(slot);
        }
        if let Some(&slot) = self.tables[frame.unit].globals.get(name) {
            return Expr::Global(slot);
        }
        let mut known: Vec<&str> = self.tables[frame.unit]
            .globals
            .keys()
            .map(String::as_str)
            .collect();
        known.extend(frame.locals.keys().map(String::as_str));
        let close = suggest(name, known).map(str::to_string);
        let mut error = Diag::name(format!("'{name}' 정의되지 않음"), span);
        if let Some(close) = close {
            error = error.with_hint(format!("비슷한 이름: '{close}'"));
        }
        self.note(frame.unit, error);
        Expr::Nothing
    }

    fn lower_expr(&mut self, frame: &mut Frame, expr: &'a ast::Expr) -> Expr {
        match expr {
            ast::Expr::Literal { value, .. } => match value {
                ast::Literal::Int(found) => Expr::Int(*found),
                ast::Literal::Float(found) => Expr::Float(*found),
                ast::Literal::Str(found) => Expr::Str(Rc::from(found.as_str())),
                ast::Literal::Bool(found) => Expr::Bool(*found),
            },
            ast::Expr::Name { name, span } => self.read_name(frame, name, *span),
            ast::Expr::List { items, .. } => Expr::List(
                items
                    .iter()
                    .map(|item| self.lower_expr(frame, item))
                    .collect(),
            ),
            ast::Expr::Dict { entries, .. } => Expr::Dict(
                entries
                    .iter()
                    .map(|(key, value)| {
                        let key = self.names.intern(key);
                        (key, self.lower_expr(frame, value))
                    })
                    .collect(),
            ),
            ast::Expr::Template { parts, .. } => Expr::Template(
                parts
                    .iter()
                    .map(|part| match part {
                        ast::TemplatePart::Text(text) => Expr::Str(Rc::from(text.as_str())),
                        ast::TemplatePart::Expr(inner) => self.lower_expr(frame, inner),
                    })
                    .collect(),
            ),
            ast::Expr::Field { owner, name, span } => {
                self.lower_field(frame, owner, name, *span)
            }
            ast::Expr::Call(call) => self.lower_call(frame, call),
            ast::Expr::Passive(passive) => self.lower_passive(frame, passive),
            ast::Expr::And { left, right, .. } => {
                let left = self.lower_expr(frame, left);
                Expr::And(Box::new(left), Box::new(self.lower_expr(frame, right)))
            }
            ast::Expr::Or { left, right, .. } => {
                let left = self.lower_expr(frame, left);
                Expr::Or(Box::new(left), Box::new(self.lower_expr(frame, right)))
            }
        }
    }

    fn lower_field(
        &mut self,
        frame: &mut Frame,
        owner: &'a ast::Expr,
        name: &str,
        span: Span,
    ) -> Expr {
        if let Some(module) = owner.as_name() {
            let known = frame.locals.contains_key(module);
            if let (false, Some(&target)) = (known, self.tables[frame.unit].modules.get(module))
            {
                if let Some(&slot) = self.tables[target].globals.get(name) {
                    return Expr::Global(slot);
                }
                let close = suggest(
                    name,
                    self.tables[target]
                        .globals
                        .keys()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )
                .map(str::to_string);
                let mut error = Diag::name(format!("모듈 '{module}'에 '{name}' 없음"), span);
                if let Some(close) = close {
                    error = error.with_hint(format!("비슷한 이름: '{close}'"));
                }
                self.note(frame.unit, error);
                return Expr::Nothing;
            }
        }
        if let Some(head) = name.strip_suffix("번째").filter(|head| !head.is_empty()) {
            let place = match head.parse::<i64>() {
                Ok(found) => Expr::Int(found),
                Err(_) => self.read_name(frame, head, span),
            };
            let owner = self.lower_expr(frame, owner);
            return Expr::Index {
                owner: Box::new(owner),
                place: Box::new(place),
                span,
            };
        }
        let owner = self.lower_expr(frame, owner);
        let field = self.names.intern(name);
        Expr::Field {
            owner: Box::new(owner),
            field,
            span,
        }
    }
}

impl<'a> Resolver<'a> {
    fn wrap(&mut self, value: Expr, call: &ast::CallExpr) -> Expr {
        let value = if call.asks {
            let verb = self.names.intern(&call.verb);
            Expr::Ask {
                value: Box::new(value),
                verb,
                span: call.span,
            }
        } else {
            value
        };
        if call.negated {
            Expr::Not(Box::new(value))
        } else {
            value
        }
    }

    fn lower_call(&mut self, frame: &mut Frame, call: &'a ast::CallExpr) -> Expr {
        let mut slots: Vec<&'a ast::Slot> = call.slots.iter().collect();
        let mut home = frame.unit;
        if let Some(index) = slots.iter().position(|slot| slot.marker == Marker::Module) {
            let slot = slots.remove(index);
            let name = slot.expr.as_name().unwrap_or_default();
            match self.tables[frame.unit].modules.get(name) {
                Some(&found) => home = found,
                None => {
                    self.note(
                        frame.unit,
                        Diag::name(format!("모듈 '{name}' 가져오지 않음"), call.span),
                    );
                    return Expr::Nothing;
                }
            }
        }
        let namespaced = home != frame.unit;
        if call.verb == "이다" {
            if let Some(found) = self.lower_predicate(frame, call, &slots, home) {
                return found;
            }
        }
        let mut args = Vec::with_capacity(slots.len());
        for slot in &slots {
            let value = self.lower_expr(frame, &slot.expr);
            args.push((slot.marker, value));
        }
        self.finish_call(frame, &call.verb, args, home, namespaced, call)
    }

    fn lower_predicate(
        &mut self,
        frame: &mut Frame,
        call: &'a ast::CallExpr,
        slots: &[&'a ast::Slot],
        home: UnitId,
    ) -> Option<Expr> {
        for (index, slot) in slots.iter().enumerate() {
            if !matches!(slot.marker, Marker::Bare | Marker::Case("가")) {
                continue;
            }
            let Some(name) = slot.expr.as_name() else {
                continue;
            };
            let predicate = format!("{name}이다");
            let rest: Vec<&'a ast::Slot> = slots
                .iter()
                .enumerate()
                .filter(|(at, other)| *at != index && other.marker != Marker::Bare)
                .map(|(_, other)| *other)
                .collect();
            let used: Vec<Marker> = rest.iter().map(|slot| slot.marker).collect();
            let found = self.tables[home]
                .verbs
                .iter()
                .rev()
                .find(|verb| verb.name == predicate && same_slots(&verb.params, &used))
                .map(|verb| (verb.func, verb.params.clone()));
            let Some((func, params)) = found else {
                continue;
            };
            let mut args = Vec::with_capacity(rest.len());
            for slot in &rest {
                let value = self.lower_expr(frame, &slot.expr);
                args.push((slot.marker, value));
            }
            let made = Expr::Call {
                callee: Callee::User(func),
                args: order_args(args, &params),
                span: call.span,
            };
            return Some(self.wrap(made, call));
        }
        None
    }

    fn finish_call(
        &mut self,
        frame: &mut Frame,
        verb: &str,
        args: Vec<(Marker, Expr)>,
        home: UnitId,
        namespaced: bool,
        call: &ast::CallExpr,
    ) -> Expr {
        let used: Vec<Marker> = args.iter().map(|(marker, _)| *marker).collect();
        let spare = format!("{verb}·나머지");
        let verb = if call.tail.as_deref() == Some("나머지") {
            spare.as_str()
        } else {
            verb
        };
        let found = self.tables[home]
            .verbs
            .iter()
            .rev()
            .find(|found| found.name == verb && same_slots(&found.params, &used))
            .map(|found| (found.func, found.params.clone()));
        if let Some((func, params)) = found {
            let made = Expr::Call {
                callee: Callee::User(func),
                args: order_args(args, &params),
                span: call.span,
            };
            return self.wrap(made, call);
        }
        if !namespaced {
            if let Some(def) = builtins::find(verb, &used) {
                let op = match (def.op, call.tail.is_some()) {
                    (builtins::Builtin::Add, true) => builtins::Builtin::AddCopy,
                    (op, _) => op,
                };
                let made = Expr::Call {
                    callee: Callee::Op(op),
                    args: order_args(args, def.params),
                    span: call.span,
                };
                return self.wrap(made, call);
            }
        }
        self.unknown_call(frame.unit, home, namespaced, verb, &used, call.span);
        Expr::Nothing
    }

    fn lower_passive(&mut self, frame: &mut Frame, passive: &'a ast::PassiveExpr) -> Expr {
        let used: Vec<Marker> = passive.slots.iter().map(|slot| slot.marker).collect();
        let found = self.tables[frame.unit]
            .verbs
            .iter()
            .rev()
            .find(|verb| {
                verb.name == passive.verb
                    && verb.params.len() == used.len() + 1
                    && crate::sig::fits(&used, &verb.params)
            })
            .map(|verb| (verb.func, verb.params.clone()));
        let Some((func, params)) = found else {
            let mut left = used.clone();
            left.push(Marker::Bare);
            self.unknown_call(
                frame.unit,
                frame.unit,
                false,
                &passive.verb,
                &left,
                passive.span,
            );
            return Expr::Nothing;
        };
        let mut empty = params.clone();
        for marker in &used {
            if let Some(at) = empty.iter().position(|kept| kept == marker) {
                empty.remove(at);
            }
        }
        let slot = empty.first().copied().unwrap_or(Marker::Bare);
        let mut args = Vec::with_capacity(params.len());
        for one in &passive.slots {
            let value = self.lower_expr(frame, &one.expr);
            args.push((one.marker, value));
        }
        let head = self.lower_expr(frame, &passive.head);
        args.push((slot, head));
        Expr::Call {
            callee: Callee::User(func),
            args: order_args(args, &params),
            span: passive.span,
        }
    }

    fn unknown_call(
        &mut self,
        unit: UnitId,
        home: UnitId,
        namespaced: bool,
        verb: &str,
        used: &[Marker],
        span: Span,
    ) {
        let mut ways: Vec<Vec<Marker>> = self.tables[home]
            .verbs
            .iter()
            .filter(|found| found.name == verb)
            .map(|found| found.params.clone())
            .collect();
        if !namespaced {
            ways.extend(builtins::ways(verb).into_iter().map(<[Marker]>::to_vec));
        }
        if ways.is_empty() {
            let mut known: Vec<&str> = self.tables[home]
                .verbs
                .iter()
                .map(|found| found.name.as_str())
                .collect();
            if !namespaced {
                known.extend(builtins::table().iter().map(|def| def.verb));
            }
            let close = suggest(verb, known).map(str::to_string);
            let mut error = Diag::name(format!("동사 '{verb}' 정의되지 않음"), span);
            if let Some(close) = close {
                error = error.with_hint(format!("비슷한 이름: '{close}'"));
            }
            return self.note(unit, error);
        }
        let listed: Vec<String> = ways.iter().map(|way| describe(verb, way)).collect();
        let error = Diag::new(
            "조사 오류",
            format!("'{verb}'를 조사 {}로 부를 수 없음", shown(used)),
            span,
        )
        .with_hint(format!("조사: {}", listed.join(" / ")));
        self.note(unit, error);
    }
}
