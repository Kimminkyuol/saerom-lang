use crate::builtins::Builtin;
use crate::diag::{Diag, Span};
use crate::hir::*;
use crate::types::{infer, Ty, Types};
use std::collections::HashMap;
use std::fmt::Write;

pub fn emit(program: &Program, triple: &str, frames: bool) -> Result<String, Vec<Diag>> {
    let types = infer(program);
    let mut emitter = Emitter::new(program, types);
    emitter.frames = frames;
    emitter.run();
    if !emitter.errors.is_empty() {
        return Err(emitter.errors);
    }
    Ok(emitter.finish(triple))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Repr {
    Word,
    Real,
    Flag,
    Boxed,
}

impl Repr {
    fn of(ty: Ty) -> Repr {
        match ty {
            Ty::Int => Repr::Word,
            Ty::Float => Repr::Real,
            Ty::Bool => Repr::Flag,
            _ => Repr::Boxed,
        }
    }

    fn llvm(self) -> &'static str {
        match self {
            Repr::Word => "i64",
            Repr::Real => "double",
            Repr::Flag => "i1",
            Repr::Boxed => "ptr",
        }
    }

    fn storage(self) -> &'static str {
        match self {
            Repr::Boxed => "%Value",
            other => other.llvm(),
        }
    }

    fn zero(self) -> &'static str {
        match self {
            Repr::Word => "0",
            Repr::Real => "0x0000000000000000",
            Repr::Flag => "false",
            Repr::Boxed => "null",
        }
    }

    fn tag(self) -> u64 {
        match self {
            Repr::Flag => 1,
            Repr::Word => 2,
            Repr::Real => 3,
            Repr::Boxed => 0,
        }
    }
}

#[derive(Clone)]
struct Val {
    repr: Repr,
    name: String,
}

struct Emitter<'a> {
    program: &'a Program,
    types: Types,
    strings: HashMap<String, String>,
    constants: String,
    functions: String,
    body: String,
    allocas: Vec<String>,
    temps: u32,
    labels: u32,
    loops: Vec<(String, String)>,
    marked: Option<u64>,
    sites: String,
    site_count: u32,
    module: ModuleId,
    current: Option<FuncId>,
    frames: bool,
    errors: Vec<Diag>,
}

struct Op {
    symbol: &'static str,
    arity: usize,
    returns: bool,
}

fn operation(op: Builtin) -> Op {
    let value = |symbol, arity| Op {
        symbol,
        arity,
        returns: true,
    };
    match op {
        Builtin::Print => Op {
            symbol: "sr_print",
            arity: 1,
            returns: false,
        },
        Builtin::Nothing => Op {
            symbol: "",
            arity: 1,
            returns: false,
        },
        Builtin::Stop => Op {
            symbol: "sr_stop",
            arity: 1,
            returns: false,
        },
        Builtin::Clone => value("sr_clone", 1),
        Builtin::Write => value("sr_write", 2),
        Builtin::Convert => value("sr_convert", 2),
        Builtin::Add => value("sr_add", 2),
        Builtin::AddCopy => value("sr_add_copy", 2),
        Builtin::Sub => value("sr_sub", 2),
        Builtin::Mul => value("sr_mul", 2),
        Builtin::Div => value("sr_div", 2),
        Builtin::Rem => value("sr_rem", 2),
        Builtin::Greater => value("sr_greater", 2),
        Builtin::Less => value("sr_less", 2),
        Builtin::Equal => value("sr_equal", 2),
        Builtin::Truthy => value("sr_truthy_value", 1),
        Builtin::Read => value("sr_read", 2),
        Builtin::Close => Op {
            symbol: "sr_close",
            arity: 1,
            returns: false,
        },
        Builtin::Open => value("sr_open", 2),
    }
}

const DECLARES: &str = "\
declare void @sr_nothing(ptr)
declare void @sr_int(ptr, i64)
declare void @sr_float(ptr, double)
declare void @sr_bool(ptr, i8)
declare void @sr_str(ptr, ptr, i64)
declare void @sr_copy(ptr, ptr)
declare i8 @sr_truthy(ptr)
declare void @sr_truthy_value(ptr, ptr)
declare void @sr_list_new(ptr)
declare void @sr_list_push(ptr, ptr)
declare i64 @sr_list_len(ptr)
declare void @sr_list_get(ptr, ptr, i64)
declare void @sr_dict_new(ptr)
declare void @sr_dict_put(ptr, ptr, i64, ptr)
declare void @sr_template(ptr, ptr, i64)
declare void @sr_field_get(ptr, ptr, ptr, i64, ptr)
declare void @sr_field_set(ptr, ptr, i64, ptr)
declare void @sr_index(ptr, ptr, ptr)
declare void @sr_range(ptr, ptr, ptr, ptr)
declare i8 @sr_name_is(ptr, i64, ptr, i64)
declare void @sr_print(ptr)
declare void @sr_add(ptr, ptr, ptr)
declare void @sr_add_copy(ptr, ptr, ptr)
declare void @sr_sub(ptr, ptr, ptr)
declare void @sr_mul(ptr, ptr, ptr)
declare void @sr_div(ptr, ptr, ptr)
declare void @sr_rem(ptr, ptr, ptr)
declare void @sr_greater(ptr, ptr, ptr)
declare void @sr_less(ptr, ptr, ptr)
declare void @sr_equal(ptr, ptr, ptr)
declare void @sr_not(ptr, ptr)
declare void @sr_convert(ptr, ptr, ptr)
declare void @sr_check_bool(ptr, ptr, i64)
declare void @sr_check_value(ptr, ptr, i64)
declare void @sr_read(ptr, ptr, ptr)
declare void @sr_open(ptr, ptr, ptr)
declare void @sr_write(ptr, ptr, ptr)
declare void @sr_close(ptr)
declare void @sr_stop(ptr)
declare void @sr_clone(ptr, ptr)
declare void @sr_finish()
declare void @sr_sources(ptr, i64, i8)
@SR_POS = external global i64
@SR_FRAMES = external global [1024 x ptr]
@SR_AT = external global [1024 x i64]
@SR_DEPTH = external global i32
";

impl<'a> Emitter<'a> {
    fn new(program: &'a Program, types: Types) -> Self {
        Emitter {
            program,
            types,
            strings: HashMap::new(),
            constants: String::new(),
            functions: String::new(),
            body: String::new(),
            allocas: Vec::new(),
            temps: 0,
            labels: 0,
            loops: Vec::new(),
            marked: None,
            sites: String::new(),
            site_count: 0,
            module: 0,
            current: None,
            frames: false,
            errors: Vec::new(),
        }
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.body, "  {text}");
    }

    fn mark(&mut self, label: &str) {
        self.marked = None;
        let _ = writeln!(self.body, "{label}:");
    }

    fn label(&mut self, tag: &str) -> String {
        self.labels += 1;
        format!("{tag}{}", self.labels)
    }

    fn temp(&mut self) -> String {
        self.temps += 1;
        format!("%v{}", self.temps)
    }

    fn slot(&mut self) -> String {
        let name = self.temp();
        self.allocas
            .push(format!("  {name} = alloca %Value, align 8"));
        name
    }

    fn raw(&mut self, kind: &str) -> String {
        let name = self.temp();
        self.allocas
            .push(format!("  {name} = alloca {kind}, align 8"));
        name
    }

    fn constant(&mut self, text: &str) -> (String, usize) {
        let len = text.len();
        if let Some(found) = self.strings.get(text) {
            return (found.clone(), len);
        }
        let name = format!("@.s{}", self.strings.len());
        let _ = writeln!(
            self.constants,
            "{name} = private unnamed_addr constant [{len} x i8] c\"{}\"",
            escape(text.as_bytes())
        );
        self.strings.insert(text.to_string(), name.clone());
        (name, len)
    }

    fn type_of(&self, expr: &Expr) -> Ty {
        self.types.of(self.current, expr)
    }

    fn field_ptr(&mut self, holder: &str, index: u32) -> String {
        let name = self.temp();
        self.line(&format!(
            "{name} = getelementptr inbounds %Value, ptr {holder}, i64 0, i32 {index}"
        ));
        name
    }

    fn boxed(&mut self, val: Val) -> String {
        if val.repr == Repr::Boxed {
            return val.name;
        }
        let bits = match val.repr {
            Repr::Word => val.name.clone(),
            Repr::Real => {
                let cast = self.temp();
                self.line(&format!("{cast} = bitcast double {} to i64", val.name));
                cast
            }
            Repr::Flag => {
                let wide = self.temp();
                self.line(&format!("{wide} = zext i1 {} to i64", val.name));
                wide
            }
            Repr::Boxed => unreachable!(),
        };
        let holder = self.slot();
        let tag = self.field_ptr(&holder, 0);
        self.line(&format!("store i64 {}, ptr {tag}, align 8", val.repr.tag()));
        let cell = self.field_ptr(&holder, 1);
        self.line(&format!("store i64 {bits}, ptr {cell}, align 8"));
        holder
    }

    fn unboxed(&mut self, val: Val, want: Repr) -> String {
        if val.repr == want {
            return val.name;
        }
        if val.repr == Repr::Boxed {
            let cell = self.field_ptr(&val.name, 1);
            let bits = self.temp();
            self.line(&format!("{bits} = load i64, ptr {cell}, align 8"));
            return match want {
                Repr::Word => bits,
                Repr::Real => {
                    let cast = self.temp();
                    self.line(&format!("{cast} = bitcast i64 {bits} to double"));
                    cast
                }
                Repr::Flag => {
                    let test = self.temp();
                    self.line(&format!("{test} = icmp ne i64 {bits}, 0"));
                    test
                }
                Repr::Boxed => unreachable!(),
            };
        }
        match (val.repr, want) {
            (Repr::Word, Repr::Real) => {
                let cast = self.temp();
                self.line(&format!("{cast} = sitofp i64 {} to double", val.name));
                cast
            }
            (Repr::Word, Repr::Flag) => {
                let test = self.temp();
                self.line(&format!("{test} = icmp ne i64 {}, 0", val.name));
                test
            }
            (Repr::Flag, Repr::Word) => {
                let wide = self.temp();
                self.line(&format!("{wide} = zext i1 {} to i64", val.name));
                wide
            }
            _ => {
                let holder = self.boxed(val);
                self.unboxed(
                    Val {
                        repr: Repr::Boxed,
                        name: holder,
                    },
                    want,
                )
            }
        }
    }

    fn truth(&mut self, val: Val) -> String {
        match val.repr {
            Repr::Flag => val.name,
            Repr::Word => self.unboxed(val, Repr::Flag),
            Repr::Real => {
                let test = self.temp();
                self.line(&format!("{test} = fcmp one double {}, 0.0", val.name));
                test
            }
            Repr::Boxed => {
                let raw = self.temp();
                self.line(&format!("{raw} = call i8 @sr_truthy(ptr {})", val.name));
                let test = self.temp();
                self.line(&format!("{test} = icmp ne i8 {raw}, 0"));
                test
            }
        }
    }

    fn packed(&self, span: Span) -> u64 {
        let width = span.end.saturating_sub(span.col).min(0xFFF);
        ((self.module as u64) << 48)
            | ((span.line as u64 & 0xFF_FFFF) << 24)
            | ((span.col as u64 & 0xFFF) << 12)
            | width as u64
    }

    fn at(&mut self, span: Span) {
        let packed = self.packed(span);
        if self.marked == Some(packed) {
            return;
        }
        self.marked = Some(packed);
        self.line(&format!("store i64 {packed}, ptr @SR_POS, align 8"));
    }

    fn nouns_argument(&self, module: ModuleId) -> String {
        if self.program.modules[module as usize].nouns.is_empty() {
            "ptr null".to_string()
        } else {
            format!("ptr @nouns_{module}")
        }
    }
}

fn escape(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &byte in bytes {
        match byte {
            b'"' | b'\\' => out.push_str(&format!("\\{byte:02X}")),
            0x20..=0x7E => out.push(byte as char),
            _ => out.push_str(&format!("\\{byte:02X}")),
        }
    }
    out
}

impl<'a> Emitter<'a> {
    fn local_repr(&self, slot: LocalId) -> Repr {
        Repr::of(self.types.place(self.current, Place::Local(slot)))
    }

    fn global_ptr(&mut self, slot: GlobalId) -> String {
        let name = self.temp();
        let count = self.program.globals.max(1);
        self.line(&format!(
            "{name} = getelementptr inbounds [{count} x %Value], ptr @globals, i64 0, i64 {slot}"
        ));
        name
    }

    fn read_place(&mut self, place: Place) -> Val {
        match place {
            Place::Local(slot) => {
                let repr = self.local_repr(slot);
                if repr == Repr::Boxed {
                    return Val {
                        repr,
                        name: format!("%l{slot}"),
                    };
                }
                let name = self.temp();
                self.line(&format!(
                    "{name} = load {}, ptr %l{slot}, align 8",
                    repr.llvm()
                ));
                Val { repr, name }
            }
            Place::Global(slot) => {
                let repr = Repr::of(self.types.globals[slot as usize]);
                let holder = self.global_ptr(slot);
                if repr == Repr::Boxed {
                    return Val { repr, name: holder };
                }
                self.unboxed_from(&holder, repr)
            }
        }
    }

    fn unboxed_from(&mut self, holder: &str, repr: Repr) -> Val {
        let boxed = Val {
            repr: Repr::Boxed,
            name: holder.to_string(),
        };
        let name = self.unboxed(boxed, repr);
        Val { repr, name }
    }

    fn write_place(&mut self, place: Place, val: Val) {
        match place {
            Place::Local(slot) => {
                let repr = self.local_repr(slot);
                if repr == Repr::Boxed {
                    let source = self.boxed(val);
                    self.line(&format!("call void @sr_copy(ptr %l{slot}, ptr {source})"));
                    return;
                }
                let found = self.unboxed(val, repr);
                self.line(&format!(
                    "store {} {found}, ptr %l{slot}, align 8",
                    repr.llvm()
                ));
            }
            Place::Global(slot) => {
                let repr = Repr::of(self.types.globals[slot as usize]);
                let holder = self.global_ptr(slot);
                if repr == Repr::Boxed {
                    let source = self.boxed(val);
                    self.line(&format!("call void @sr_copy(ptr {holder}, ptr {source})"));
                    return;
                }
                let bits = match repr {
                    Repr::Word => self.unboxed(val, Repr::Word),
                    Repr::Real => {
                        let found = self.unboxed(val, Repr::Real);
                        let cast = self.temp();
                        self.line(&format!("{cast} = bitcast double {found} to i64"));
                        cast
                    }
                    Repr::Flag => {
                        let found = self.unboxed(val, Repr::Flag);
                        let wide = self.temp();
                        self.line(&format!("{wide} = zext i1 {found} to i64"));
                        wide
                    }
                    Repr::Boxed => unreachable!(),
                };
                let tag = self.field_ptr(&holder, 0);
                self.line(&format!("store i64 {}, ptr {tag}, align 8", repr.tag()));
                let cell = self.field_ptr(&holder, 1);
                self.line(&format!("store i64 {bits}, ptr {cell}, align 8"));
            }
        }
    }

    fn value(&mut self, expr: &'a Expr, want: Repr) -> String {
        let found = self.expr(expr);
        self.unboxed(found, want)
    }

    fn signature(&self, id: FuncId) -> (Repr, Vec<Repr>) {
        let found = &self.program.functions[id as usize];
        let ret = Repr::of(self.types.returns[id as usize]);
        let params = found
            .params
            .iter()
            .map(|&slot| Repr::of(self.types.locals[id as usize][slot as usize]))
            .collect();
        (ret, params)
    }

    fn expr(&mut self, expr: &'a Expr) -> Val {
        match expr {
            Expr::Int(found) => Val {
                repr: Repr::Word,
                name: found.to_string(),
            },
            Expr::Float(found) => Val {
                repr: Repr::Real,
                name: format!("0x{:016X}", found.to_bits()),
            },
            Expr::Bool(found) => Val {
                repr: Repr::Flag,
                name: found.to_string(),
            },
            Expr::Nothing => {
                let out = self.slot();
                self.line(&format!("call void @sr_nothing(ptr {out})"));
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Str(found) => {
                let out = self.slot();
                let (name, len) = self.constant(found);
                self.line(&format!(
                    "call void @sr_str(ptr {out}, ptr {name}, i64 {len})"
                ));
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Local(slot) => self.read_place(Place::Local(*slot)),
            Expr::Global(slot) => self.read_place(Place::Global(*slot)),
            Expr::List(items) => {
                let out = self.slot();
                self.line(&format!("call void @sr_list_new(ptr {out})"));
                for item in items {
                    let value = self.expr(item);
                    let held = self.boxed(value);
                    self.line(&format!("call void @sr_list_push(ptr {out}, ptr {held})"));
                }
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Dict(entries) => {
                let out = self.slot();
                self.line(&format!("call void @sr_dict_new(ptr {out})"));
                for (key, value) in entries {
                    let value = self.expr(value);
                    let held = self.boxed(value);
                    let (name, len) = self.name_of(*key);
                    self.line(&format!(
                        "call void @sr_dict_put(ptr {out}, ptr {name}, i64 {len}, ptr {held})"
                    ));
                }
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Template(parts) => {
                let count = parts.len();
                let array = self.raw(&format!("[{count} x %Value]"));
                for (index, part) in parts.iter().enumerate() {
                    let value = self.expr(part);
                    let held = self.boxed(value);
                    let cell = self.temp();
                    self.line(&format!(
                        "{cell} = getelementptr inbounds [{count} x %Value], ptr {array}, i64 0, i64 {index}"
                    ));
                    self.line(&format!("call void @sr_copy(ptr {cell}, ptr {held})"));
                }
                let out = self.slot();
                self.line(&format!(
                    "call void @sr_template(ptr {out}, ptr {array}, i64 {count})"
                ));
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Field { owner, field, span } => {
                if let Some(found) = self.direct_noun(owner, *field, *span) {
                    return found;
                }
                let owner = self.expr(owner);
                let owner = self.boxed(owner);
                let (name, len) = self.name_of(*field);
                let nouns = self.nouns_argument(self.module);
                self.at(*span);
                let out = self.slot();
                self.line(&format!(
                    "call void @sr_field_get(ptr {out}, ptr {owner}, ptr {name}, i64 {len}, {nouns})"
                ));
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Index { owner, place, span } => {
                let owner = self.expr(owner);
                let owner = self.boxed(owner);
                let place = self.expr(place);
                let place = self.boxed(place);
                self.at(*span);
                let out = self.slot();
                self.line(&format!(
                    "call void @sr_index(ptr {out}, ptr {owner}, ptr {place})"
                ));
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Call { callee, args, span } => self.call(*callee, args, *span),
            Expr::Not(value) => {
                let found = self.expr(value);
                let flag = self.truth(found);
                let out = self.temp();
                self.line(&format!("{out} = xor i1 {flag}, true"));
                Val {
                    repr: Repr::Flag,
                    name: out,
                }
            }
            Expr::Ask { value, verb, span } => {
                let found = self.expr(value);
                if found.repr == Repr::Flag {
                    return found;
                }
                let held = self.boxed(found);
                let (name, len) = self.name_of(*verb);
                self.at(*span);
                self.line(&format!(
                    "call void @sr_check_bool(ptr {held}, ptr {name}, i64 {len})"
                ));
                self.unboxed_from(&held.clone(), Repr::Flag)
            }
            Expr::And(left, right) => self.shortcut(left, right, true),
            Expr::Or(left, right) => self.shortcut(left, right, false),
        }
    }

    fn direct_noun(
        &mut self,
        owner: &'a Expr,
        field: crate::intern::Symbol,
        span: Span,
    ) -> Option<Val> {
        let held = self.type_of(owner);
        if !matches!(held, Ty::Int | Ty::Float | Ty::Bool) {
            return None;
        }
        let name = self.program.names.name(field);
        if name == "복사본" || name == "자료형" {
            return None;
        }
        let func = *self.program.modules[self.module as usize]
            .nouns
            .get(&field)?;
        let (ret, params) = self.signature(func);
        let want = params.first().copied().unwrap_or(Repr::Boxed);
        let value = self.expr(owner);
        let given = if want == Repr::Boxed {
            let held = self.boxed(value);
            format!("ptr {held}")
        } else {
            let held = self.unboxed(value, want);
            format!("{} {held}", want.llvm())
        };
        self.at(span);
        let label = self.program.names.name(field).to_string();
        let depth = self.enter(&label, Some(span));
        let made = if ret == Repr::Boxed {
            let out = self.slot();
            self.line(&format!("call void @fn_{func}(ptr {out}, {given})"));
            Val {
                repr: Repr::Boxed,
                name: out,
            }
        } else {
            let out = self.temp();
            self.line(&format!("{out} = call {} @fn_{func}({given})", ret.llvm()));
            Val {
                repr: ret,
                name: out,
            }
        };
        if let Some(depth) = depth {
            self.line(&format!("store i32 {depth}, ptr @SR_DEPTH, align 4"));
        }
        self.guard_result(func, &made, span);
        Some(made)
    }

    fn name_of(&mut self, field: crate::intern::Symbol) -> (String, usize) {
        let text = self.program.names.name(field).to_string();
        self.constant(&text)
    }

    fn shortcut(&mut self, left: &'a Expr, right: &'a Expr, all: bool) -> Val {
        let out = self.raw("i1");
        let value = self.expr(left);
        let taken = self.truth(value);
        let other = self.label("rhs");
        let quick = self.label("quick");
        let end = self.label("joined");
        let (yes, no) = if all {
            (&other, &quick)
        } else {
            (&quick, &other)
        };
        self.line(&format!("br i1 {taken}, label %{yes}, label %{no}"));
        self.mark(&other.clone());
        let value = self.expr(right);
        let seen = self.truth(value);
        self.line(&format!("store i1 {seen}, ptr {out}, align 1"));
        self.line(&format!("br label %{end}"));
        self.mark(&quick.clone());
        self.line(&format!("store i1 {}, ptr {out}, align 1", !all));
        self.line(&format!("br label %{end}"));
        self.mark(&end);
        let name = self.temp();
        self.line(&format!("{name} = load i1, ptr {out}, align 1"));
        Val {
            repr: Repr::Flag,
            name,
        }
    }
}

impl<'a> Emitter<'a> {
    fn inline_op(&mut self, op: Builtin, args: &'a [Expr]) -> Option<Val> {
        if args.len() != 2 {
            if op == Builtin::Truthy && args.len() == 1 && self.type_of(&args[0]) == Ty::Bool {
                return Some(self.expr(&args[0]));
            }
            return None;
        }
        let left = self.type_of(&args[0]);
        let right = self.type_of(&args[1]);
        let whole = left == Ty::Int && right == Ty::Int;
        let real = left.number() && right.number() && !whole;

        let arith = |name: &'static str, floating: &'static str| Some((name, floating));
        let found = match op {
            Builtin::Add | Builtin::AddCopy => arith("add", "fadd"),
            Builtin::Sub => arith("sub", "fsub"),
            Builtin::Mul => arith("mul", "fmul"),
            _ => None,
        };
        if let Some((whole_op, real_op)) = found {
            if whole {
                let a = self.value(&args[0], Repr::Word);
                let b = self.value(&args[1], Repr::Word);
                let out = self.temp();
                self.line(&format!("{out} = {whole_op} i64 {a}, {b}"));
                return Some(Val {
                    repr: Repr::Word,
                    name: out,
                });
            }
            if real {
                let a = self.value(&args[0], Repr::Real);
                let b = self.value(&args[1], Repr::Real);
                let out = self.temp();
                self.line(&format!("{out} = {real_op} double {a}, {b}"));
                return Some(Val {
                    repr: Repr::Real,
                    name: out,
                });
            }
            return None;
        }

        let compare = match op {
            Builtin::Greater => Some(("sgt", "ogt")),
            Builtin::Less => Some(("slt", "olt")),
            Builtin::Equal => Some(("eq", "oeq")),
            _ => None,
        }?;
        if whole || (op == Builtin::Equal && left == Ty::Bool && right == Ty::Bool) {
            let want = if whole { Repr::Word } else { Repr::Flag };
            let a = self.value(&args[0], want);
            let b = self.value(&args[1], want);
            let out = self.temp();
            self.line(&format!(
                "{out} = icmp {} {} {a}, {b}",
                compare.0,
                want.llvm()
            ));
            return Some(Val {
                repr: Repr::Flag,
                name: out,
            });
        }
        if real {
            let a = self.value(&args[0], Repr::Real);
            let b = self.value(&args[1], Repr::Real);
            let out = self.temp();
            self.line(&format!("{out} = fcmp {} double {a}, {b}", compare.1));
            return Some(Val {
                repr: Repr::Flag,
                name: out,
            });
        }
        None
    }

    fn call(&mut self, callee: Callee, args: &'a [Expr], span: Span) -> Val {
        if let Callee::Op(op) = callee {
            if let Some(found) = self.inline_op(op, args) {
                return found;
            }
        }
        match callee {
            Callee::Op(op) => self.op_call(op, args, span),
            Callee::User(func) => self.user_call(func, args, span),
        }
    }

    fn op_call(&mut self, op: Builtin, args: &'a [Expr], span: Span) -> Val {
        let mut given = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.expr(arg);
            given.push(self.boxed(value));
        }
        self.at(span);
        let found = operation(op);
        let out = self.slot();
        if found.symbol.is_empty() {
            self.line(&format!("call void @sr_nothing(ptr {out})"));
            return Val {
                repr: Repr::Boxed,
                name: out,
            };
        }
        let mut passed: Vec<String> = given
            .iter()
            .take(found.arity)
            .map(|name| format!("ptr {name}"))
            .collect();
        if found.returns {
            passed.insert(0, format!("ptr {out}"));
        } else {
            self.line(&format!("call void @sr_nothing(ptr {out})"));
        }
        self.line(&format!(
            "call void @{}({})",
            found.symbol,
            passed.join(", ")
        ));
        Val {
            repr: Repr::Boxed,
            name: out,
        }
    }

    fn user_call(&mut self, func: FuncId, args: &'a [Expr], span: Span) -> Val {
        let (ret, params) = self.signature(func);
        let mut passed = Vec::with_capacity(args.len());
        for (index, arg) in args.iter().enumerate() {
            let want = params.get(index).copied().unwrap_or(Repr::Boxed);
            let value = self.expr(arg);
            let name = if want == Repr::Boxed {
                self.boxed(value)
            } else {
                self.unboxed(value, want)
            };
            passed.push(format!("{} {name}", want.llvm()));
        }
        self.at(span);
        let name = self.program.functions[func as usize].name.to_string();
        let depth = self.enter(&name, Some(span));
        let made = if ret == Repr::Boxed {
            let out = self.slot();
            let mut all = vec![format!("ptr {out}")];
            all.extend(passed);
            self.line(&format!("call void @fn_{func}({})", all.join(", ")));
            Val {
                repr: Repr::Boxed,
                name: out,
            }
        } else {
            let out = self.temp();
            self.line(&format!(
                "{out} = call {} @fn_{func}({})",
                ret.llvm(),
                passed.join(", ")
            ));
            Val {
                repr: ret,
                name: out,
            }
        };
        if let Some(depth) = depth {
            self.line(&format!("store i32 {depth}, ptr @SR_DEPTH, align 4"));
        }
        self.guard_result(func, &made, span);
        made
    }

    fn guard_result(&mut self, func: FuncId, made: &Val, span: Span) {
        let kind = self.program.functions[func as usize].kind;
        let ty = self.types.returns[func as usize];
        let needed = match kind {
            Kind::Noun => matches!(ty, Ty::Any | Ty::Nothing | Ty::Never),
            Kind::Verb => false,
        };
        if !needed {
            return;
        }
        let name = self.program.functions[func as usize].name.to_string();
        let held = self.boxed(made.clone());
        let (text, len) = self.constant(&name);
        self.at(span);
        let symbol = "sr_check_value";
        self.line(&format!(
            "call void @{symbol}(ptr {held}, ptr {text}, i64 {len})"
        ));
    }
}

impl<'a> Emitter<'a> {
    fn statement(&mut self, statement: &'a Stmt) {
        match statement {
            Stmt::Set { place, value } => {
                let value = self.expr(value);
                self.write_place(*place, value);
            }
            Stmt::SetField {
                owner,
                field,
                value,
                span,
            } => {
                let owner = self.expr(owner);
                let owner = self.boxed(owner);
                let value = self.expr(value);
                let value = self.boxed(value);
                let (name, len) = self.name_of(*field);
                self.at(*span);
                self.line(&format!(
                    "call void @sr_field_set(ptr {owner}, ptr {name}, i64 {len}, ptr {value})"
                ));
            }
            Stmt::Eval(value) => {
                self.expr(value);
            }
            Stmt::If {
                branches,
                otherwise,
            } => self.if_chain(branches, otherwise.as_deref()),
            Stmt::Range {
                place,
                start,
                stop,
                step,
                body,
                span,
            } => self.range(*place, start, stop, step.as_ref(), body, *span),
            Stmt::While { test, body } => self.while_loop(test, body),
            Stmt::Break | Stmt::Continue => {
                let Some((step, end)) = self.loops.last().cloned() else {
                    return;
                };
                let target = if matches!(statement, Stmt::Break) {
                    end
                } else {
                    step
                };
                self.line(&format!("br label %{target}"));
                let next = self.label("dead");
                self.mark(&next);
            }
            Stmt::Return { value, .. } => {
                let value = self.expr(value);
                match self.current.map(|id| self.signature(id).0) {
                    Some(Repr::Boxed) | None => {
                        let held = self.boxed(value);
                        self.line(&format!("call void @sr_copy(ptr %out, ptr {held})"));
                        self.line("ret void");
                    }
                    Some(ret) => {
                        let found = self.unboxed(value, ret);
                        self.line(&format!("ret {} {found}", ret.llvm()));
                    }
                }
                let next = self.label("dead");
                self.mark(&next);
            }
        }
    }

    fn if_chain(&mut self, branches: &'a [(Expr, Vec<Stmt>)], otherwise: Option<&'a [Stmt]>) {
        let end = self.label("endif");
        for (test, body) in branches {
            let value = self.expr(test);
            let taken = self.truth(value);
            let then = self.label("then");
            let next = self.label("elif");
            self.line(&format!("br i1 {taken}, label %{then}, label %{next}"));
            self.mark(&then);
            for statement in body {
                self.statement(statement);
            }
            self.line(&format!("br label %{end}"));
            self.mark(&next);
        }
        if let Some(body) = otherwise {
            for statement in body {
                self.statement(statement);
            }
        }
        self.line(&format!("br label %{end}"));
        self.mark(&end);
    }

    fn while_loop(&mut self, test: &'a Expr, body: &'a [Stmt]) {
        let head = self.label("while");
        let inside = self.label("do");
        let end = self.label("done");
        self.line(&format!("br label %{head}"));
        self.mark(&head);
        let value = self.expr(test);
        let taken = self.truth(value);
        self.line(&format!("br i1 {taken}, label %{inside}, label %{end}"));
        self.mark(&inside);
        self.loops.push((head.clone(), end.clone()));
        for statement in body {
            self.statement(statement);
        }
        self.loops.pop();
        self.line(&format!("br label %{head}"));
        self.mark(&end);
    }

    fn range(
        &mut self,
        place: Place,
        start: &'a Expr,
        stop: &'a Expr,
        step: Option<&'a Expr>,
        body: &'a [Stmt],
        span: Span,
    ) {
        let whole = self.type_of(start) == Ty::Int
            && self.type_of(stop) == Ty::Int
            && step.is_none_or(|step| self.type_of(step) == Ty::Int);
        if whole {
            return self.counted(place, start, stop, step, body);
        }
        let start = self.expr(start);
        let start = self.boxed(start);
        let stop = self.expr(stop);
        let stop = self.boxed(stop);
        let step = match step {
            Some(found) => {
                let found = self.expr(found);
                self.boxed(found)
            }
            None => {
                let one = self.slot();
                self.line(&format!("call void @sr_int(ptr {one}, i64 1)"));
                one
            }
        };
        let list = self.slot();
        self.at(span);
        self.line(&format!(
            "call void @sr_range(ptr {list}, ptr {start}, ptr {stop}, ptr {step})"
        ));
        let count = self.temp();
        self.line(&format!("{count} = call i64 @sr_list_len(ptr {list})"));
        let index = self.raw("i64");
        self.line(&format!("store i64 0, ptr {index}, align 8"));

        let head = self.label("for");
        let inside = self.label("body");
        let next = self.label("next");
        let end = self.label("endfor");
        self.line(&format!("br label %{head}"));
        self.mark(&head);
        let now = self.temp();
        self.line(&format!("{now} = load i64, ptr {index}, align 8"));
        let more = self.temp();
        self.line(&format!("{more} = icmp slt i64 {now}, {count}"));
        self.line(&format!("br i1 {more}, label %{inside}, label %{end}"));
        self.mark(&inside);
        let item = self.slot();
        self.line(&format!(
            "call void @sr_list_get(ptr {item}, ptr {list}, i64 {now})"
        ));
        self.write_place(
            place,
            Val {
                repr: Repr::Boxed,
                name: item,
            },
        );
        self.loops.push((next.clone(), end.clone()));
        for statement in body {
            self.statement(statement);
        }
        self.loops.pop();
        self.line(&format!("br label %{next}"));
        self.mark(&next);
        let seen = self.temp();
        self.line(&format!("{seen} = load i64, ptr {index}, align 8"));
        let bumped = self.temp();
        self.line(&format!("{bumped} = add i64 {seen}, 1"));
        self.line(&format!("store i64 {bumped}, ptr {index}, align 8"));
        self.line(&format!("br label %{head}"));
        self.mark(&end);
    }

    fn counted(
        &mut self,
        place: Place,
        start: &'a Expr,
        stop: &'a Expr,
        step: Option<&'a Expr>,
        body: &'a [Stmt],
    ) {
        let from = self.value(start, Repr::Word);
        let to = self.value(stop, Repr::Word);
        let by = match step {
            Some(found) => self.value(found, Repr::Word),
            None => "1".to_string(),
        };
        let negated = self.temp();
        self.line(&format!("{negated} = sub i64 0, {by}"));
        let downward = self.temp();
        self.line(&format!("{downward} = icmp slt i64 {by}, 0"));
        let size = self.temp();
        self.line(&format!(
            "{size} = select i1 {downward}, i64 {negated}, i64 {by}"
        ));
        let idle = self.temp();
        self.line(&format!("{idle} = icmp eq i64 {size}, 0"));
        let width = self.temp();
        self.line(&format!("{width} = select i1 {idle}, i64 1, i64 {size}"));
        let back = self.temp();
        self.line(&format!("{back} = icmp sgt i64 {from}, {to}"));
        let fall = self.temp();
        self.line(&format!("{fall} = sub i64 0, {width}"));
        let delta = self.temp();
        self.line(&format!(
            "{delta} = select i1 {back}, i64 {fall}, i64 {width}"
        ));

        let counter = self.raw("i64");
        self.line(&format!("store i64 {from}, ptr {counter}, align 8"));
        let head = self.label("for");
        let inside = self.label("body");
        let next = self.label("next");
        let end = self.label("endfor");
        self.line(&format!("br label %{head}"));
        self.mark(&head);
        let now = self.temp();
        self.line(&format!("{now} = load i64, ptr {counter}, align 8"));
        let rising = self.temp();
        self.line(&format!("{rising} = icmp sle i64 {now}, {to}"));
        let falling = self.temp();
        self.line(&format!("{falling} = icmp sge i64 {now}, {to}"));
        let more = self.temp();
        self.line(&format!(
            "{more} = select i1 {back}, i1 {falling}, i1 {rising}"
        ));
        self.line(&format!("br i1 {more}, label %{inside}, label %{end}"));
        self.mark(&inside);
        self.write_place(
            place,
            Val {
                repr: Repr::Word,
                name: now.clone(),
            },
        );
        self.loops.push((next.clone(), end.clone()));
        for statement in body {
            self.statement(statement);
        }
        self.loops.pop();
        self.line(&format!("br label %{next}"));
        self.mark(&next);
        let seen = self.temp();
        self.line(&format!("{seen} = load i64, ptr {counter}, align 8"));
        let bumped = self.temp();
        self.line(&format!("{bumped} = add i64 {seen}, {delta}"));
        self.line(&format!("store i64 {bumped}, ptr {counter}, align 8"));
        self.line(&format!("br label %{head}"));
        self.mark(&end);
    }
}

impl<'a> Emitter<'a> {
    fn run(&mut self) {
        for id in 0..self.program.modules.len() as ModuleId {
            self.noun_table(id);
        }
        for (index, function) in self.program.functions.iter().enumerate() {
            self.function(index as FuncId, function);
        }
        for id in 0..self.program.modules.len() as ModuleId {
            self.module_init(id);
        }
    }

    fn open(&mut self, module: ModuleId) {
        self.body.clear();
        self.allocas.clear();
        self.temps = 0;
        self.module = module;
        self.marked = None;
        self.loops.clear();
    }

    fn close(&mut self, header: &str, prologue: &str, ret: Repr) {
        let _ = writeln!(self.functions, "{header} {{\nentry:");
        for line in &self.allocas {
            let _ = writeln!(self.functions, "{line}");
        }
        self.functions.push_str(prologue);
        self.functions.push_str(&self.body);
        let tail = match ret {
            Repr::Boxed => "  ret void".to_string(),
            other => format!("  ret {} {}", other.llvm(), other.zero()),
        };
        let _ = writeln!(self.functions, "{tail}\n}}\n");
    }

    fn enter(&mut self, name: &str, at: Option<Span>) -> Option<String> {
        if !self.frames {
            return None;
        }
        let (text, len) = self.constant(name);
        let site = format!("@.site{}", self.site_count);
        self.site_count += 1;
        let _ = writeln!(
            self.sites,
            "{site} = private constant %Site {{ ptr {text}, i64 {len} }}"
        );
        let here = match at {
            Some(span) => self.packed(span).to_string(),
            None => {
                let loaded = self.temp();
                self.line(&format!("{loaded} = load i64, ptr @SR_POS, align 8"));
                loaded
            }
        };
        let depth = self.temp();
        self.line(&format!("{depth} = load i32, ptr @SR_DEPTH, align 4"));
        let wrapped = self.temp();
        self.line(&format!("{wrapped} = and i32 {depth}, 1023"));
        let cell = self.temp();
        self.line(&format!(
            "{cell} = getelementptr inbounds [1024 x ptr], ptr @SR_FRAMES, i64 0, i32 {wrapped}"
        ));
        self.line(&format!("store ptr {site}, ptr {cell}, align 8"));
        let mark = self.temp();
        self.line(&format!(
            "{mark} = getelementptr inbounds [1024 x i64], ptr @SR_AT, i64 0, i32 {wrapped}"
        ));
        self.line(&format!("store i64 {here}, ptr {mark}, align 8"));
        let next = self.temp();
        self.line(&format!("{next} = add i32 {depth}, 1"));
        self.line(&format!("store i32 {next}, ptr @SR_DEPTH, align 4"));
        Some(depth)
    }

    fn noun_table(&mut self, module: ModuleId) {
        let mut pairs: Vec<(String, FuncId)> = self.program.modules[module as usize]
            .nouns
            .iter()
            .map(|(&field, &func)| (self.program.names.name(field).to_string(), func))
            .collect();
        if pairs.is_empty() {
            return;
        }
        pairs.sort();
        self.open(module);
        self.current = None;
        for (field, func) in pairs {
            let (text, len) = self.constant(&field);
            let same = self.temp();
            self.line(&format!(
                "{same} = call i8 @sr_name_is(ptr %name, i64 %len, ptr {text}, i64 {len})"
            ));
            let test = self.temp();
            self.line(&format!("{test} = icmp ne i8 {same}, 0"));
            let hit = self.label("hit");
            let miss = self.label("miss");
            self.line(&format!("br i1 {test}, label %{hit}, label %{miss}"));
            self.mark(&hit);
            let depth = self.enter(&field, None);
            let (ret, params) = self.signature(func);
            let want = params.first().copied().unwrap_or(Repr::Boxed);
            let owner = if want == Repr::Boxed {
                "ptr %owner".to_string()
            } else {
                let found = self.unboxed(
                    Val {
                        repr: Repr::Boxed,
                        name: "%owner".into(),
                    },
                    want,
                );
                format!("{} {found}", want.llvm())
            };
            if ret == Repr::Boxed {
                self.line(&format!("call void @fn_{func}(ptr %out, {owner})"));
            } else {
                let made = self.temp();
                self.line(&format!("{made} = call {} @fn_{func}({owner})", ret.llvm()));
                let held = self.boxed(Val {
                    repr: ret,
                    name: made,
                });
                self.line(&format!("call void @sr_copy(ptr %out, ptr {held})"));
            }
            if let Some(depth) = depth {
                self.line(&format!("store i32 {depth}, ptr @SR_DEPTH, align 4"));
            }
            self.line("ret i8 1");
            self.mark(&miss);
        }
        self.line("ret i8 0");
        let header = format!(
            "define internal i8 @nouns_{module}(ptr %out, ptr %owner, ptr %name, i64 %len)"
        );
        let _ = writeln!(self.functions, "{header} {{\nentry:");
        let allocas = self.allocas.join("\n");
        if !allocas.is_empty() {
            let _ = writeln!(self.functions, "{allocas}");
        }
        let body = std::mem::take(&mut self.body);
        self.functions.push_str(&body);
        let _ = writeln!(self.functions, "}}\n");
    }

    fn function(&mut self, id: FuncId, function: &'a Function) {
        self.open(function.module);
        self.current = Some(id);
        let (ret, params) = self.signature(id);
        let mut prologue = String::new();
        for slot in 0..function.locals {
            let repr = Repr::of(self.types.locals[id as usize][slot as usize]);
            self.allocas
                .push(format!("  %l{slot} = alloca {}, align 8", repr.storage()));
            if repr == Repr::Boxed {
                let _ = writeln!(prologue, "  call void @sr_nothing(ptr %l{slot})");
            } else {
                let _ = writeln!(
                    prologue,
                    "  store {} {}, ptr %l{slot}, align 8",
                    repr.llvm(),
                    repr.zero()
                );
            }
        }
        if ret == Repr::Boxed {
            let _ = writeln!(prologue, "  call void @sr_nothing(ptr %out)");
        }
        for (index, slot) in function.params.iter().enumerate() {
            let repr = params[index];
            if repr == Repr::Boxed {
                let _ = writeln!(
                    prologue,
                    "  call void @sr_copy(ptr %l{slot}, ptr %p{index})"
                );
            } else {
                let _ = writeln!(
                    prologue,
                    "  store {} %p{index}, ptr %l{slot}, align 8",
                    repr.llvm()
                );
            }
        }
        for statement in &function.body {
            self.statement(statement);
        }
        let shown: Vec<String> = params
            .iter()
            .enumerate()
            .map(|(index, repr)| format!("{} %p{index}", repr.llvm()))
            .collect();
        let header = if ret == Repr::Boxed {
            let mut all = vec!["ptr %out".to_string()];
            all.extend(shown);
            format!("define internal void @fn_{id}({})", all.join(", "))
        } else {
            format!(
                "define internal {} @fn_{id}({})",
                ret.llvm(),
                shown.join(", ")
            )
        };
        self.close(&header, &prologue, ret);
        self.current = None;
    }

    fn module_init(&mut self, id: ModuleId) {
        self.open(id);
        self.current = None;
        let module = &self.program.modules[id as usize];
        for statement in &module.init {
            self.statement(statement);
        }
        let header = format!("define internal void @mod_{id}()");
        self.close(&header, "", Repr::Boxed);
    }

    fn finish(&mut self, triple: &str) -> String {
        let traced = self.frames as u8;
        let mut table = Vec::new();
        for id in 0..self.program.modules.len() {
            let module = &self.program.modules[id];
            let (path, path_len) = self.constant(module.path.as_ref());
            let (text, text_len) = self.constant(module.source.as_ref());
            table.push(format!(
                "%Source {{ ptr {path}, i64 {path_len}, ptr {text}, i64 {text_len} }}"
            ));
        }
        let count = table.len();
        let mut out = String::new();
        if !triple.is_empty() {
            let _ = writeln!(out, "target triple = \"{triple}\"\n");
        }
        let _ = writeln!(out, "%Value = type {{ i64, i64 }}");
        let _ = writeln!(out, "%Source = type {{ ptr, i64, ptr, i64 }}");
        let _ = writeln!(out, "%Site = type {{ ptr, i64 }}\n");
        let _ = writeln!(
            out,
            "@globals = internal global [{} x %Value] zeroinitializer\n",
            self.program.globals.max(1)
        );
        out.push_str(&self.constants);
        out.push_str(&self.sites);
        let _ = writeln!(
            out,
            "@sources = internal constant [{count} x %Source] [{}]",
            table.join(", ")
        );
        out.push('\n');
        out.push_str(DECLARES);
        out.push('\n');
        out.push_str(&self.functions);
        let _ = writeln!(out, "define i32 @main() {{\nentry:");
        let _ = writeln!(
            out,
            "  call void @sr_sources(ptr @sources, i64 {count}, i8 {traced})"
        );
        for id in &self.program.order {
            let _ = writeln!(out, "  call void @mod_{id}()");
        }
        let _ = writeln!(out, "  call void @sr_finish()");
        let _ = writeln!(out, "  ret i32 0\n}}");
        out
    }
}
