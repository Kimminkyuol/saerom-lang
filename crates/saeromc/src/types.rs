use crate::builtins::Builtin;
use crate::hir::*;
use crate::intern::Symbol;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ty {
    Never,
    Nothing,
    Bool,
    Int,
    Float,
    Str,
    Table,
    Any,
}

impl Ty {
    pub fn join(self, other: Ty) -> Ty {
        match (self, other) {
            (a, b) if a == b => a,
            (Ty::Never, b) => b,
            (a, Ty::Never) => a,
            _ => Ty::Any,
        }
    }

    pub fn number(self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }
}

#[derive(Default)]
pub struct Fields {
    kind: Option<Symbol>,
    length: Option<Symbol>,
    names: Option<Symbol>,
}

pub struct Types {
    pub fields: Fields,
    pub nouns: HashMap<Symbol, Vec<FuncId>>,
    pub globals: Vec<Ty>,
    pub locals: Vec<Vec<Ty>>,
    pub returns: Vec<Ty>,
    pub constants: Vec<Option<Rc<str>>>,
}

impl Types {
    pub fn place(&self, function: Option<FuncId>, place: Place) -> Ty {
        match place {
            Place::Global(slot) => self.globals[slot as usize],
            Place::Local(slot) => match function {
                Some(id) => self.locals[id as usize][slot as usize],
                None => Ty::Any,
            },
        }
    }

    pub fn of(&self, function: Option<FuncId>, expr: &Expr) -> Ty {
        match expr {
            Expr::Int(_) => Ty::Int,
            Expr::Float(_) => Ty::Float,
            Expr::Str(_) => Ty::Str,
            Expr::Bool(_) => Ty::Bool,
            Expr::Nothing => Ty::Nothing,
            Expr::Local(slot) => self.place(function, Place::Local(*slot)),
            Expr::Global(slot) => self.globals[*slot as usize],
            Expr::Table(..) => Ty::Table,
            Expr::Template(_) => Ty::Str,
            Expr::Field { owner, field, .. } => self.field(function, owner, *field),
            Expr::Index { .. } | Expr::Pick { .. } => Ty::Any,
            Expr::Call { callee, args, .. } => self.call(function, *callee, args),
            Expr::Not(_) | Expr::Ask { .. } | Expr::And(..) | Expr::Or(..) => Ty::Bool,
        }
    }

    fn field(&self, function: Option<FuncId>, owner: &Expr, field: Symbol) -> Ty {
        let held = self.of(function, owner);
        let found = Some(field);
        if found == self.fields.kind {
            return Ty::Str;
        }
        if found == self.fields.length && matches!(held, Ty::Table | Ty::Str) {
            return Ty::Int;
        }
        if found == self.fields.names && held == Ty::Table {
            return Ty::Table;
        }
        if let Some(candidates) = self.nouns.get(&field) {
            if matches!(held, Ty::Int | Ty::Float | Ty::Bool) {
                return candidates.iter().fold(Ty::Never, |so_far, &id| {
                    so_far.join(self.returns[id as usize])
                });
            }
        }
        Ty::Any
    }

    fn call(&self, function: Option<FuncId>, callee: Callee, args: &[Expr]) -> Ty {
        let op = match callee {
            Callee::User(id) => return self.returns[id as usize],
            Callee::Op(op) => op,
        };
        let arg = |index: usize| args.get(index).map_or(Ty::Never, |e| self.of(function, e));
        match op {
            Builtin::Print | Builtin::Stop | Builtin::Nothing => Ty::Nothing,
            Builtin::Greater | Builtin::Less | Builtin::Equal | Builtin::Truthy => Ty::Bool,
            // 실패도 파일 끝도 없음이라 갈래가 섞인다.
            Builtin::Read | Builtin::Open | Builtin::Write => Ty::Any,
            Builtin::Close => Ty::Nothing,
            Builtin::Clone => arg(0),
            Builtin::Convert => self.converted(function, args),
            Builtin::Add => match (arg(0), arg(1)) {
                (Ty::Str, _) => Ty::Str,
                (a, b) => arithmetic(a, b),
            },
            Builtin::Push | Builtin::RemoveAt | Builtin::RemoveKey => Ty::Nothing,
            Builtin::Sub | Builtin::Mul | Builtin::Rem => arithmetic(arg(0), arg(1)),
            // 나누다는 늘 실수. 몫은 정수끼리면 정수.
            Builtin::Div => Ty::Float,
            Builtin::Quot => arithmetic(arg(0), arg(1)),
        }
    }

    fn converted(&self, function: Option<FuncId>, args: &[Expr]) -> Ty {
        let kind = match args.get(1) {
            Some(Expr::Str(text)) => Some(text.clone()),
            Some(Expr::Global(slot)) => self.constants[*slot as usize].clone(),
            _ => None,
        };
        let _ = function;
        match kind.as_deref() {
            Some("정수") | Some("수") => Ty::Int,
            Some("실수") => Ty::Float,
            Some("문자열") => Ty::Str,
            Some("논리값") => Ty::Bool,
            _ => Ty::Any,
        }
    }
}

fn arithmetic(left: Ty, right: Ty) -> Ty {
    match (left, right) {
        (Ty::Int, Ty::Int) => Ty::Int,
        (a, b) if a.number() && b.number() => Ty::Float,
        _ => Ty::Any,
    }
}

pub fn infer(program: &Program) -> Types {
    let named = |wanted: &str| {
        (0..program.names.len() as Symbol).find(|&id| program.names.name(id) == wanted)
    };
    let fields = Fields {
        kind: named("자료형"),
        length: named("길이"),
        names: named("명칭"),
    };
    let mut nouns: HashMap<Symbol, Vec<FuncId>> = HashMap::new();
    for module in &program.modules {
        for (&field, &id) in &module.nouns {
            nouns.entry(field).or_default().push(id);
        }
    }
    let mut types = Types {
        fields,
        nouns,
        globals: vec![Ty::Never; program.globals as usize],
        locals: program
            .functions
            .iter()
            .map(|found| vec![Ty::Never; found.locals as usize])
            .collect(),
        returns: vec![Ty::Never; program.functions.len()],
        constants: constants_of(program),
    };
    for (id, function) in program.functions.iter().enumerate() {
        if !always_returns(&function.body) {
            types.returns[id] = Ty::Nothing;
        }
    }
    // 격자 높이가 3(Never ⊑ 구체 ⊑ Any)이고 갱신이 모두 join 이라 반드시 멎는다.
    // 반복 상한을 두면 고정점에 못 닿은 채 좁은 타입이 남아 코드 생성이 잘못된 표현으로
    // 언박싱한다. 상한을 두지 않는 것이 건전성의 조건이다.
    loop {
        let mut moved = false;
        for module in &program.modules {
            walk(&mut types, program, None, &module.init, &mut moved);
        }
        for (id, function) in program.functions.iter().enumerate() {
            walk(
                &mut types,
                program,
                Some(id as FuncId),
                &function.body,
                &mut moved,
            );
        }
        if !moved {
            break;
        }
    }
    types
}

fn constants_of(program: &Program) -> Vec<Option<Rc<str>>> {
    let mut found: Vec<Option<Rc<str>>> = vec![None; program.globals as usize];
    let mut counts = vec![0usize; program.globals as usize];
    for module in &program.modules {
        count_sets(&module.init, &mut counts, &mut found);
    }
    for function in &program.functions {
        count_sets(&function.body, &mut counts, &mut found);
    }
    found
        .into_iter()
        .zip(counts)
        .map(|(text, count)| if count == 1 { text } else { None })
        .collect()
}

fn count_sets(body: &[Stmt], counts: &mut [usize], found: &mut [Option<Rc<str>>]) {
    for statement in body {
        if let Stmt::Set {
            place: Place::Global(slot),
            value,
        } = statement
        {
            counts[*slot as usize] += 1;
            if let Expr::Str(text) = value {
                found[*slot as usize] = Some(text.clone());
            }
        }
        if let Stmt::Range {
            place: Place::Global(slot),
            ..
        } = statement
        {
            counts[*slot as usize] += 2;
        }
        for block in blocks(statement) {
            count_sets(block, counts, found);
        }
    }
}

fn blocks(statement: &Stmt) -> Vec<&[Stmt]> {
    match statement {
        Stmt::If {
            branches,
            otherwise,
        } => {
            let mut found: Vec<&[Stmt]> =
                branches.iter().map(|(_, body)| body.as_slice()).collect();
            found.extend(otherwise.iter().map(Vec::as_slice));
            found
        }
        Stmt::Range { body, .. } | Stmt::While { body, .. } | Stmt::Each { body, .. } => {
            vec![body]
        }
        _ => Vec::new(),
    }
}

fn raise(slot: &mut Ty, found: Ty, moved: &mut bool) {
    let joined = slot.join(found);
    if joined != *slot {
        *slot = joined;
        *moved = true;
    }
}

fn walk(
    types: &mut Types,
    program: &Program,
    function: Option<FuncId>,
    body: &[Stmt],
    moved: &mut bool,
) {
    for statement in body {
        match statement {
            Stmt::Set { place, value } => {
                let found = types.of(function, value);
                assign(types, function, *place, found, moved);
                visit(types, program, function, value, moved);
            }
            Stmt::SetField { owner, value, .. } => {
                visit(types, program, function, owner, moved);
                visit(types, program, function, value, moved);
            }
            Stmt::SetAt {
                owner,
                place,
                value,
                ..
            } => {
                visit(types, program, function, owner, moved);
                visit(types, program, function, place, moved);
                visit(types, program, function, value, moved);
            }
            Stmt::Each {
                place, over, body, ..
            } => {
                assign(types, function, *place, Ty::Any, moved);
                visit(types, program, function, over, moved);
                walk(types, program, function, body, moved);
            }
            Stmt::SetPick {
                owner, key, value, ..
            } => {
                visit(types, program, function, owner, moved);
                visit(types, program, function, key, moved);
                visit(types, program, function, value, moved);
            }
            Stmt::Eval(value) => visit(types, program, function, value, moved),
            Stmt::If {
                branches,
                otherwise,
            } => {
                for (test, block) in branches {
                    visit(types, program, function, test, moved);
                    walk(types, program, function, block, moved);
                }
                if let Some(block) = otherwise {
                    walk(types, program, function, block, moved);
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
                let mut found = types.of(function, start).join(types.of(function, stop));
                if let Some(step) = step {
                    found = found.join(types.of(function, step));
                }
                let element = if found == Ty::Int {
                    Ty::Int
                } else if found.number() {
                    Ty::Float
                } else {
                    Ty::Any
                };
                assign(types, function, *place, element, moved);
                visit(types, program, function, start, moved);
                visit(types, program, function, stop, moved);
                if let Some(step) = step {
                    visit(types, program, function, step, moved);
                }
                walk(types, program, function, body, moved);
            }
            Stmt::While { test, body } => {
                visit(types, program, function, test, moved);
                walk(types, program, function, body, moved);
            }
            Stmt::Break | Stmt::Continue => {}
            Stmt::Return { value, .. } => {
                let found = types.of(function, value);
                if let Some(id) = function {
                    raise(&mut types.returns[id as usize], found, moved);
                }
                visit(types, program, function, value, moved);
            }
        }
    }
}

fn assign(
    types: &mut Types,
    function: Option<FuncId>,
    place: Place,
    found: Ty,
    moved: &mut bool,
) {
    match place {
        Place::Global(slot) => raise(&mut types.globals[slot as usize], found, moved),
        Place::Local(slot) => {
            if let Some(id) = function {
                raise(&mut types.locals[id as usize][slot as usize], found, moved);
            }
        }
    }
}

fn visit(
    types: &mut Types,
    program: &Program,
    function: Option<FuncId>,
    expr: &Expr,
    moved: &mut bool,
) {
    match expr {
        Expr::Template(items) => {
            for item in items {
                visit(types, program, function, item, moved);
            }
        }
        Expr::Table(items, entries) => {
            for item in items {
                visit(types, program, function, item, moved);
            }
            for (_, value) in entries {
                visit(types, program, function, value, moved);
            }
        }
        Expr::Field { owner, field, .. } => {
            visit(types, program, function, owner, moved);
            let held = types.of(function, owner);
            if let Some(candidates) = types.nouns.get(field).cloned() {
                for id in candidates {
                    let Some(&slot) = program.functions[id as usize].params.first() else {
                        continue;
                    };
                    raise(&mut types.locals[id as usize][slot as usize], held, moved);
                }
            }
        }
        Expr::Index { owner, place, .. } => {
            visit(types, program, function, owner, moved);
            visit(types, program, function, place, moved);
        }
        Expr::Pick { owner, key, .. } => {
            visit(types, program, function, owner, moved);
            visit(types, program, function, key, moved);
        }
        Expr::Not(inner) | Expr::Ask { value: inner, .. } => {
            visit(types, program, function, inner, moved)
        }
        Expr::And(left, right) | Expr::Or(left, right) => {
            visit(types, program, function, left, moved);
            visit(types, program, function, right, moved);
        }
        Expr::Call { callee, args, .. } => {
            for arg in args {
                visit(types, program, function, arg, moved);
            }
            if let Callee::User(id) = callee {
                let params = program.functions[*id as usize].params.clone();
                for (slot, arg) in params.into_iter().zip(args) {
                    let found = types.of(function, arg);
                    raise(&mut types.locals[*id as usize][slot as usize], found, moved);
                }
            }
        }
        _ => {}
    }
}

// 흐름이 아래로 안 이어지는가. `종료한다`는 되돌아오지 않으므로 반환과 같다.
pub fn always_returns(body: &[Stmt]) -> bool {
    let Some(last) = body.last() else {
        return false;
    };
    match last {
        Stmt::Return { .. } => true,
        Stmt::Eval(Expr::Call {
            callee: Callee::Op(Builtin::Stop),
            ..
        }) => true,
        Stmt::If {
            branches,
            otherwise: Some(otherwise),
        } => {
            branches.iter().all(|(_, block)| always_returns(block)) && always_returns(otherwise)
        }
        _ => false,
    }
}
