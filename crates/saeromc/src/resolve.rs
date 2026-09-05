use crate::ast::{self, LoopKind, Stmt as ASt};
use crate::builtins;
use crate::diag::{suggest, Diag, Span};
use crate::hir::*;
use crate::intern::Interner;
use crate::load::{Loaded, UnitId};
use crate::msg;
use crate::sig::{describe, shown, Marker};
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
    // 동사 자리 매개변수 -> 실제로 넘어온 동사 이름
    verbs: HashMap<String, String>,
}

// 동사를 매개변수로 받는 정의. 호출 자리마다 복제해서 푼다.
struct Generic {
    params: Vec<(Marker, String)>,
    verbs: Vec<String>,
}

impl Frame {
    fn place(&mut self, name: &str, tables: &mut Tables, globals: &mut u32) -> Place {
        if self.inside_function {
            // 함수 안의 매김은 언제나 지역이다. 전역으로 새면 이름이 겹치는 것만으로
            // 남의 전역(모듈 상수 포함)을 덮어쓴다.
            let _ = tables;
            return Place::Local(self.bind_local(name));
        }
        Place::Global(global_slot(name, tables, globals))
    }

    fn bind_local(&mut self, name: &str) -> LocalId {
        if let Some(&found) = self.locals.get(name) {
            return found;
        }
        let slot = self.count;
        self.count += 1;
        self.locals.insert(name.to_string(), slot);
        slot
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
    generics: HashMap<FuncId, Generic>,
    special: HashMap<(FuncId, Vec<String>), FuncId>,
    bindings: Vec<HashMap<String, String>>,
}

const MAX_SPECIAL: usize = 256;

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
        generics: HashMap::new(),
        special: HashMap::new(),
        bindings: Vec::new(),
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
    resolver.spread_verb_slots();
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

fn spot_of(name: &str) -> Option<&str> {
    name.strip_suffix("번째").filter(|head| !head.is_empty())
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

        for statement in statements {
            if let ASt::Import {
                names: None, path, ..
            } = statement
            {
                if let Some(target) = loaded.unit_of(path) {
                    self.take_globals(unit, target);
                }
            }
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
            self.take_definitions(unit, target);
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
                let mut error = Diag::name(msg::module_lacks(module, name), span);
                if let Some(close) = close {
                    error = error.with_hint(msg::similar(&close));
                }
                self.note(unit, error);
            }
        }
    }

    fn take_definitions(&mut self, unit: UnitId, target: UnitId) {
        let verbs: Vec<Verb> = self.tables[target]
            .verbs
            .iter()
            .map(|found| Verb {
                name: found.name.clone(),
                params: found.params.clone(),
                func: found.func,
            })
            .collect();
        let nouns: Vec<(String, FuncId)> = self.tables[target]
            .nouns
            .iter()
            .map(|(name, &func)| (name.clone(), func))
            .collect();
        self.tables[unit].verbs.extend(verbs);
        self.tables[unit].nouns.extend(nouns);
    }

    fn take_globals(&mut self, unit: UnitId, target: UnitId) {
        let globals: Vec<(String, GlobalId)> = self.tables[target]
            .globals
            .iter()
            .map(|(name, &slot)| (name.clone(), slot))
            .collect();
        for (name, slot) in globals {
            self.tables[unit].globals.entry(name).or_insert(slot);
        }
    }

    fn declare_functions(&mut self, unit: UnitId, statements: &'a [ASt]) {
        for statement in statements {
            match statement {
                ASt::Define {
                    name,
                    params,
                    body,
                    span,
                } => {
                    if crate::sig::Signatures::reserved(name) {
                        self.note(unit, Diag::name(msg::builtin_reserved(name), *span));
                        continue;
                    }
                    // `철수의 스물을 알린다`는 파서가 `철수의 스물`을 필드로 먼저
                    // 먹어 호출이 성립하지 않는다. 인자가 리터럴일 때만 우연히
                    // 되던 자리라 아예 막는다.
                    if params.iter().any(|(marker, _)| *marker == Marker::Case("의")) {
                        self.note(
                            unit,
                            Diag::syntax(msg::GENITIVE_PARAM.to_string(), *span)
                        );
                        continue;
                    }
                    let func = self.functions.len() as FuncId;
                    self.functions.push(Function {
                        name: Rc::from(name.as_str()),
                        kind: Kind::Verb,
                        module: self.module_of[unit],
                        params: Vec::new(),
                        locals: 0,
                        body: Vec::new(),
                        span: *span,
                    });
                    self.bodies.push(statement);
                    self.owners.push(unit);
                    self.bindings.push(HashMap::new());
                    let taken = verb_slots(params, body);
                    if !taken.is_empty() {
                        self.generics.insert(
                            func,
                            Generic {
                                params: params.clone(),
                                verbs: taken,
                            },
                        );
                    }
                    let params: Vec<Marker> = params.iter().map(|(marker, _)| *marker).collect();
                    // 같은 이름·같은 조사면 호출 때 나중 것이 조용히 이긴다.
                    if self.tables[unit]
                        .verbs
                        .iter()
                        .any(|found| found.name == *name && same_slots(&found.params, &params))
                    {
                        self.note(unit, Diag::name(msg::already_defined(name), *span));
                    }
                    self.tables[unit].verbs.push(Verb {
                        name: name.clone(),
                        params,
                        func,
                    });
                }
                ASt::Noun { name, span, .. } => {
                    if crate::words::FIELDS.contains(&name.as_str()) {
                        self.note(unit, Diag::name(msg::builtin_reserved(name), *span));
                        continue;
                    }
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
                    self.bindings.push(HashMap::new());
                    if self.tables[unit].nouns.contains_key(name) {
                        self.note(unit, Diag::name(msg::already_defined(name), *span));
                    }
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

// 매개변수 이름을 몸통에서 동사로 부르면 그 자리는 동사 자리다.
fn verb_slots(params: &[(Marker, String)], body: &[ASt]) -> Vec<String> {
    let mut called = Vec::new();
    calls_in_block(body, &mut called);
    params
        .iter()
        .filter(|(_, name)| called.iter().any(|call| call.verb == *name))
        .map(|(_, name)| name.clone())
        .collect()
}

fn calls_in_block<'b>(body: &'b [ASt], into: &mut Vec<&'b ast::CallExpr>) {
    for statement in body {
        match statement {
            ASt::Declare { assigns, .. } => {
                for (_, value) in assigns {
                    calls_in_expr(value, into);
                }
            }
            ASt::Exec { calls, .. } => {
                for call in calls {
                    into.push(call);
                    for slot in &call.slots {
                        calls_in_expr(&slot.expr, into);
                    }
                }
            }
            ASt::Value { expr, .. } | ASt::Return { value: expr, .. } => {
                calls_in_expr(expr, into)
            }
            ASt::If { branches, .. } => {
                for (test, _) in branches {
                    calls_in_expr(test, into);
                }
            }
            ASt::Loop { kind, .. } => match kind {
                LoopKind::Range {
                    start, stop, step, ..
                } => {
                    calls_in_expr(start, into);
                    calls_in_expr(stop, into);
                    if let Some(step) = step {
                        calls_in_expr(step, into);
                    }
                }
                LoopKind::While { test } => calls_in_expr(test, into),
                LoopKind::Each { over, .. } => calls_in_expr(over, into),
            },
            _ => {}
        }
        for block in blocks_of(statement) {
            calls_in_block(block, into);
        }
    }
}

fn calls_in_expr<'b>(expr: &'b ast::Expr, into: &mut Vec<&'b ast::CallExpr>) {
    match expr {
        ast::Expr::Call(call) => {
            into.push(call);
            for slot in &call.slots {
                calls_in_expr(&slot.expr, into);
            }
        }
        ast::Expr::Passive(passive) => {
            calls_in_expr(&passive.head, into);
            for slot in &passive.slots {
                calls_in_expr(&slot.expr, into);
            }
        }
        ast::Expr::Table { items, entries, .. } => {
            for item in items {
                calls_in_expr(item, into);
            }
            for (_, value) in entries {
                calls_in_expr(value, into);
            }
        }
        ast::Expr::Template { parts, .. } => {
            for part in parts {
                if let ast::TemplatePart::Expr(inner) = part {
                    calls_in_expr(inner, into);
                }
            }
        }
        ast::Expr::Field { owner, .. } => calls_in_expr(owner, into),
        ast::Expr::Pick { owner, key, .. } => {
            calls_in_expr(owner, into);
            calls_in_expr(key, into);
        }
        ast::Expr::Spot { owner, place, .. } => {
            calls_in_expr(owner, into);
            calls_in_expr(place, into);
        }
        ast::Expr::And { left, right, .. } | ast::Expr::Or { left, right, .. } => {
            calls_in_expr(left, into);
            calls_in_expr(right, into);
        }
        _ => {}
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
        ASt::Loop { body, .. } => vec![body],
        _ => Vec::new(),
    }
}

fn bind_names(statements: &[ASt], into: &mut Vec<String>) {
    for statement in statements {
        match statement {
            ASt::Declare { assigns, .. } => into.extend(
                assigns
                    .iter()
                    .filter(|(target, _)| target.fields.is_empty())
                    .map(|(target, _)| target.root.clone()),
            ),
            ASt::Loop {
                kind: LoopKind::Range { variable, .. } | LoopKind::Each { variable, .. },
                ..
            } => into.push(variable.clone()),
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
        frame.bind_local(name)
    }

    fn lower_module(&mut self, unit: UnitId) -> Module {
        let loaded = self.loaded;
        let statements: &'a [ASt] = &loaded.units[unit].statements;
        let mut frame = Frame {
            unit,
            locals: HashMap::new(),
            count: 0,
            inside_function: false,
            verbs: HashMap::new(),
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
            unit,
            name: Rc::from(found.name.as_str()),
            path: Rc::from(path.as_str()),
            source: Rc::from(found.source.as_str()),
            init,
            nouns,
        }
    }

    // 매개변수를 부르지 않고 다른 동사 자리로 넘기기만 해도 동사 자리다.
    // 넘겨받는 쪽이 나중에 정해질 수 있으므로 고정점까지 돈다.
    fn spread_verb_slots(&mut self) {
        loop {
            let mut moved = false;
            for index in 0..self.bodies.len() {
                let ASt::Define { params, body, .. } = self.bodies[index] else {
                    continue;
                };
                let unit = self.owners[index];
                let mut calls = Vec::new();
                calls_in_block(body, &mut calls);
                for call in calls {
                    let used: Vec<Marker> =
                        call.slots.iter().map(|slot| slot.marker).collect();
                    let Some(target) = self.tables[unit]
                        .verbs
                        .iter()
                        .rev()
                        .find(|found| found.name == call.verb && same_slots(&found.params, &used))
                        .map(|found| found.func)
                    else {
                        continue;
                    };
                    let Some(generic) = self.generics.get(&target) else {
                        continue;
                    };
                    let wanted: Vec<Marker> = generic
                        .params
                        .iter()
                        .filter(|(_, name)| generic.verbs.contains(name))
                        .map(|(marker, _)| *marker)
                        .collect();
                    for marker in wanted {
                        let Some(slot) = call.slots.iter().find(|slot| slot.marker == marker)
                        else {
                            continue;
                        };
                        let Some(given) = slot.expr.as_name() else { continue };
                        if !params.iter().any(|(_, name)| name == given) {
                            continue;
                        }
                        let here = index as FuncId;
                        let found = self.generics.entry(here).or_insert_with(|| Generic {
                            params: params.clone(),
                            verbs: Vec::new(),
                        });
                        if !found.verbs.iter().any(|name| name == given) {
                            found.verbs.push(given.to_string());
                            moved = true;
                        }
                    }
                }
            }
            if !moved {
                return;
            }
        }
    }

    fn lower_functions(&mut self) -> Vec<Function> {
        let mut index = 0;
        // 특수화가 뒤에 붙으므로 길이를 매번 다시 본다.
        while index < self.bodies.len() {
            let statement = self.bodies[index];
            let unit = self.owners[index];
            let mut frame = Frame {
                unit,
                locals: HashMap::new(),
                count: 0,
                inside_function: true,
                verbs: self.bindings[index].clone(),
            };
            // 동사 자리가 채워지지 않은 틀은 부를 수 없다. 특수화한 것만 낮춘다.
            if self.generics.contains_key(&(index as FuncId)) && frame.verbs.is_empty() {
                index += 1;
                continue;
            }
            let (params, body) = match statement {
                ASt::Define { params, body, .. } => {
                    let wanted: Vec<&String> = params
                        .iter()
                        .map(|(_, name)| name)
                        .filter(|name| !frame.verbs.contains_key(*name))
                        .collect();
                    let slots = wanted
                        .into_iter()
                        .map(|name| self.local(&mut frame, name))
                        .collect();
                    (slots, body)
                }
                ASt::Noun { owner, body, .. } => (vec![self.local(&mut frame, owner)], body),
                _ => continue,
            };
            let mut bound = Vec::new();
            bind_names(body, &mut bound);
            for name in bound {
                frame.place(&name, &mut self.tables[unit], &mut self.globals);
            }
            let taken: std::collections::HashSet<String> = frame.locals.keys().cloned().collect();
            let given: std::collections::HashSet<String> = match statement {
                ASt::Define { params, .. } => params
                    .iter()
                    .map(|(_, name)| name.clone())
                    .filter(|name| !frame.verbs.contains_key(name))
                    .collect(),
                ASt::Noun { owner, .. } => std::iter::once(owner.clone()).collect(),
                _ => Default::default(),
            };
            for error in crate::assign::check(body, &taken, &given) {
                self.note(unit, error);
            }
            let lowered = self.lower_block(&mut frame, body);
            self.functions[index].params = params;
            self.functions[index].locals = frame.count;
            self.functions[index].body = lowered;
            index += 1;
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
            ASt::Declare { assigns, span } => {
                for (target, value) in assigns {
                    let value = self.lower_expr(frame, value);
                    if target.fields.is_empty() {
                        let place = frame.place(
                            &target.root,
                            &mut self.tables[frame.unit],
                            &mut self.globals,
                        );
                        out.push(Stmt::Set { place, value });
                        continue;
                    }
                    let mut owner = self.read_name(frame, &target.root, target.span);
                    for field in &target.fields[..target.fields.len() - 1] {
                        owner = match field {
                            ast::Selector::Name(name) => {
                                let field = self.names.intern(name);
                                Expr::Field {
                                    owner: Box::new(owner),
                                    field,
                                    span: target.span,
                                }
                            }
                            ast::Selector::Pick(key) => {
                                let key = self.lower_expr(frame, key);
                                Expr::Pick {
                                    owner: Box::new(owner),
                                    key: Box::new(key),
                                    span: target.span,
                                }
                            }
                            ast::Selector::Spot(place) => {
                                let place = self.lower_expr(frame, place);
                                Expr::Index {
                                    owner: Box::new(owner),
                                    place: Box::new(place),
                                    span: target.span,
                                }
                            }
                        };
                    }
                    match target.fields.last().expect("필드") {
                        ast::Selector::Name(name) => match spot_of(name) {
                            Some(head) => {
                                let place = self.spot_place(frame, head, target.span);
                                out.push(Stmt::SetAt {
                                    owner,
                                    place,
                                    value,
                                    span: *span,
                                });
                            }
                            None => {
                                let field = self.names.intern(name);
                                out.push(Stmt::SetField {
                                    owner,
                                    field,
                                    value,
                                    span: *span,
                                });
                            }
                        },
                        ast::Selector::Spot(place) => {
                            let place = self.lower_expr(frame, place);
                            out.push(Stmt::SetAt {
                                owner,
                                place,
                                value,
                                span: *span,
                            });
                        }
                        ast::Selector::Pick(key) => {
                            let key = self.lower_expr(frame, key);
                            out.push(Stmt::SetPick {
                                owner,
                                key,
                                value,
                                span: *span,
                            });
                        }
                    }
                }
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
                LoopKind::Each { variable, over } => {
                    let over = self.lower_expr(frame, over);
                    let place =
                        frame.place(variable, &mut self.tables[frame.unit], &mut self.globals);
                    let body = self.lower_block(frame, body);
                    out.push(Stmt::Each {
                        place,
                        over,
                        body,
                        span: *span,
                    });
                }
            },
            ASt::Break { .. } => out.push(Stmt::Break),
            ASt::Continue { .. } => out.push(Stmt::Continue),
            ASt::Return { value, span } => {
                let value = self.lower_expr(frame, value);
                out.push(Stmt::Return { value, span: *span });
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
        let mut error = Diag::name(msg::undefined(name), span);
        if let Some(close) = close {
            error = error.with_hint(msg::similar(&close));
        }
        self.note(frame.unit, error);
        Expr::Nothing
    }

    fn spot_place(&mut self, frame: &mut Frame, head: &str, span: Span) -> Expr {
        match head.parse::<i64>() {
            Ok(found) => Expr::Int(found),
            Err(_) => self.read_name(frame, head, span),
        }
    }

    fn lower_expr(&mut self, frame: &mut Frame, expr: &'a ast::Expr) -> Expr {
        match expr {
            ast::Expr::Literal { value, .. } => match value {
                ast::Literal::Nothing => Expr::Nothing,
                ast::Literal::Int(found) => Expr::Int(*found),
                ast::Literal::Float(found) => Expr::Float(*found),
                ast::Literal::Str(found) => Expr::Str(Rc::from(found.as_str())),
                ast::Literal::Bool(found) => Expr::Bool(*found),
            },
            ast::Expr::Name { name, span } => self.read_name(frame, name, *span),
            ast::Expr::Table { items, entries, .. } => Expr::Table(
                items
                    .iter()
                    .map(|item| self.lower_expr(frame, item))
                    .collect(),
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
            ast::Expr::Pick { owner, key, span } => {
                let owner = self.lower_expr(frame, owner);
                let key = self.lower_expr(frame, key);
                Expr::Pick {
                    owner: Box::new(owner),
                    key: Box::new(key),
                    span: *span,
                }
            }
            ast::Expr::Spot { owner, place, span } => {
                let owner = self.lower_expr(frame, owner);
                let place = self.lower_expr(frame, place);
                Expr::Index {
                    owner: Box::new(owner),
                    place: Box::new(place),
                    span: *span,
                }
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
                let mut error = Diag::name(msg::module_lacks(module, name), span);
                if let Some(close) = close {
                    error = error.with_hint(msg::similar(&close));
                }
                self.note(frame.unit, error);
                return Expr::Nothing;
            }
        }
        if let Some(head) = spot_of(name) {
            let place = self.spot_place(frame, head, span);
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
                        Diag::name(msg::module_not_taken(name), call.span),
                    );
                    return Expr::Nothing;
                }
            }
        }
        let namespaced = home != frame.unit;
        if !namespaced && call.verb == "제거하다" {
            return self.lower_remove(frame, call, &slots);
        }
        if call.verb == "이다" {
            if let Some(found) = self.lower_predicate(frame, call, &slots, home) {
                return found;
            }
        }
        // 동사 자리에 온 이름은 값이 아니다. 인자를 낮추기 전에 걷어낸다.
        let bound = match self.verb_bindings(frame, &call.verb, &slots, home, call.span) {
            Ok(found) => found,
            Err(()) => return Expr::Nothing,
        };
        let mut args = Vec::with_capacity(slots.len());
        for slot in &slots {
            if bound.iter().any(|(marker, _, _)| *marker == slot.marker) {
                continue;
            }
            let value = self.lower_expr(frame, &slot.expr);
            args.push((slot.marker, value));
        }
        self.finish_call(frame, &call.verb, args, home, namespaced, call, &bound)
    }

    fn lower_remove(
        &mut self,
        frame: &mut Frame,
        call: &'a ast::CallExpr,
        slots: &[&'a ast::Slot],
    ) -> Expr {
        let target = slots
            .iter()
            .find(|slot| slot.marker == Marker::Case("를"))
            .map(|slot| &slot.expr);
        if let Some(ast::Expr::Pick { owner, key, span }) = target {
            let owner = self.lower_expr(frame, owner);
            let key = self.lower_expr(frame, key);
            return Expr::Call {
                callee: Callee::Op(builtins::Builtin::RemoveKey),
                args: vec![owner, key],
                span: *span,
            };
        }
        if let Some(ast::Expr::Spot { owner, place, span }) = target {
            let owner = self.lower_expr(frame, owner);
            let place = self.lower_expr(frame, place);
            return Expr::Call {
                callee: Callee::Op(builtins::Builtin::RemoveAt),
                args: vec![owner, place],
                span: *span,
            };
        }
        let Some(ast::Expr::Field { owner, name, span }) = target else {
            self.note(frame.unit, Diag::syntax(msg::REMOVE_NEEDS_FIELD, call.span));
            return Expr::Nothing;
        };
        let (op, place) = match self.lower_field(frame, owner, name, *span) {
            Expr::Index { owner, place, .. } => (Callee::Op(builtins::Builtin::RemoveAt), {
                let mut args = vec![*owner];
                args.push(*place);
                args
            }),
            Expr::Field { owner, field, .. } => (
                Callee::Op(builtins::Builtin::RemoveKey),
                vec![*owner, Expr::Str(Rc::from(self.names.name(field)))],
            ),
            _ => {
                self.note(frame.unit, Diag::syntax(msg::REMOVE_NEEDS_FIELD, call.span));
                return Expr::Nothing;
            }
        };
        Expr::Call {
            callee: op,
            args: place,
            span: call.span,
        }
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

    // 이 호출이 동사 매개변수를 받는 정의를 가리키면, 그 자리와 넘어온 동사 이름을 낸다.
    fn verb_bindings(
        &mut self,
        frame: &Frame,
        verb: &str,
        slots: &[&'a ast::Slot],
        home: UnitId,
        span: Span,
    ) -> std::result::Result<Vec<(Marker, String, String)>, ()> {
        let used: Vec<Marker> = slots.iter().map(|slot| slot.marker).collect();
        let Some(func) = self.tables[home]
            .verbs
            .iter()
            .rev()
            .find(|found| found.name == verb && same_slots(&found.params, &used))
            .map(|found| found.func)
        else {
            return Ok(Vec::new());
        };
        let Some(generic) = self.generics.get(&func) else {
            return Ok(Vec::new());
        };
        let wanted: Vec<(Marker, String)> = generic
            .params
            .iter()
            .filter(|(_, name)| generic.verbs.contains(name))
            .cloned()
            .collect();
        let mut found = Vec::new();
        for (marker, name) in wanted {
            let Some(slot) = slots.iter().find(|slot| slot.marker == marker) else {
                continue;
            };
            let Some(given) = slot.expr.as_name() else {
                self.note(
                    home,
                    Diag::syntax(msg::want_verb_name(&name), slot.expr.span()),
                );
                return Err(());
            };
            // 동사 자리를 다시 넘기는 꼴이면 이미 묶인 이름으로 바꾼다.
            let given: &str = frame.verbs.get(given).map_or(given, String::as_str);
            if !self.tables[home]
                .verbs
                .iter()
                .any(|kept| kept.name == given)
            {
                let close = suggest(
                    given,
                    self.tables[home]
                        .verbs
                        .iter()
                        .map(|kept| kept.name.as_str())
                        .collect::<Vec<_>>(),
                )
                .map(str::to_string);
                let mut error = Diag::name(msg::verb_undefined(given), slot.expr.span());
                if let Some(close) = close {
                    error = error.with_hint(msg::similar(&close));
                }
                self.note(home, error);
                return Err(());
            }
            found.push((marker, name, given.to_string()));
        }
        let _ = span;
        Ok(found)
    }

    // 넘어온 동사 이름마다 몸통을 한 벌씩 만든다.
    fn specialize(&mut self, func: FuncId, given: &[(String, String)]) -> Option<FuncId> {
        let key: Vec<String> = given.iter().map(|(_, verb)| verb.clone()).collect();
        if let Some(&found) = self.special.get(&(func, key.clone())) {
            return Some(found);
        }
        if self.special.len() >= MAX_SPECIAL {
            let span = self.functions[func as usize].span;
            self.note(self.owners[func as usize], Diag::syntax(msg::TOO_MANY_SPECIAL, span));
            return None;
        }
        let made = self.functions.len() as FuncId;
        let source = &self.functions[func as usize];
        self.functions.push(Function {
            name: source.name.clone(),
            kind: source.kind,
            module: source.module,
            params: Vec::new(),
            locals: 0,
            body: Vec::new(),
            span: source.span,
        });
        self.bodies.push(self.bodies[func as usize]);
        self.owners.push(self.owners[func as usize]);
        self.bindings
            .push(given.iter().cloned().collect::<HashMap<_, _>>());
        self.special.insert((func, key), made);
        Some(made)
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_call(
        &mut self,
        frame: &mut Frame,
        verb: &str,
        args: Vec<(Marker, Expr)>,
        home: UnitId,
        namespaced: bool,
        call: &ast::CallExpr,
        given: &[(Marker, String, String)],
    ) -> Expr {
        // 동사 자리는 인자에서 빠졌으니 찾을 때만 도로 넣는다.
        let mut used: Vec<Marker> = args.iter().map(|(marker, _)| *marker).collect();
        used.extend(given.iter().map(|(marker, _, _)| *marker));
        // `나눈 나머지`·`나눈 몫`처럼 꼬리 명사가 갈래를 가른다.
        let spare = call
            .tail
            .as_deref()
            .filter(|tail| *tail != "값")
            .map(|tail| format!("{verb}·{tail}"));
        let verb = spare.as_deref().unwrap_or(verb);
        // 동사 자리 매개변수면 넘어온 이름으로 바꿔 부른다.
        let swapped = frame.verbs.get(verb).cloned();
        let verb = swapped.as_deref().unwrap_or(verb);
        let found = self.tables[home]
            .verbs
            .iter()
            .rev()
            .find(|found| found.name == verb && same_slots(&found.params, &used))
            .map(|found| (found.func, found.params.clone()));
        if let Some((func, params)) = found {
            let (func, params) = if given.is_empty() {
                (func, params)
            } else {
                let bind: Vec<(String, String)> = given
                    .iter()
                    .map(|(_, name, verb)| (name.clone(), verb.clone()))
                    .collect();
                let Some(made) = self.specialize(func, &bind) else {
                    return Expr::Nothing;
                };
                let left: Vec<Marker> = params
                    .into_iter()
                    .filter(|marker| !given.iter().any(|(kept, _, _)| kept == marker))
                    .collect();
                (made, left)
            };
            let made = Expr::Call {
                callee: Callee::User(func),
                args: order_args(args, &params),
                span: call.span,
            };
            return self.wrap(made, call);
        }
        if !namespaced {
            if let Some(def) = builtins::find(verb, &used) {
                let made = Expr::Call {
                    callee: Callee::Op(def.op),
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
            let mut error = Diag::name(msg::verb_undefined(verb), span);
            if let Some(close) = close {
                error = error.with_hint(msg::similar(&close));
            }
            return self.note(unit, error);
        }
        let listed: Vec<String> = ways.iter().map(|way| describe(verb, way)).collect();
        let error = Diag::new(
            msg::PARTICLE,
            msg::wrong_particles(verb, &shown(used)),
            span,
        )
        .with_hint(msg::ways(&listed.join(" / ")));
        self.note(unit, error);
    }
}
