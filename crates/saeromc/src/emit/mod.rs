use crate::builtins::Builtin;
use crate::diag::{Diag, Span};
use crate::hir::*;
use crate::types::{infer, Ty, Types};
use std::collections::HashMap;
use std::fmt::Write;

pub fn emit(program: &Program, triple: &str, frames: bool) -> Result<String, Vec<Diag>> {
    let types = infer(program);
    let reuse = crate::reuse::find(program, &types);
    let mut emitter = Emitter::new(program, types, reuse);
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
    reuse: crate::reuse::Reuse,
    strings: HashMap<String, String>,
    constants: String,
    functions: String,
    body: String,
    allocas: Vec<String>,
    roots: Vec<String>,
    temps: u32,
    labels: u32,
    loops: Vec<(String, String)>,
    marked: Option<u64>,
    sites: String,
    site_count: u32,
    cached: std::collections::HashSet<String>,
    mutable_str: bool,
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
        Builtin::Push => Op {
            symbol: "sr_push",
            arity: 2,
            returns: false,
        },
        Builtin::RemoveAt => Op {
            symbol: "sr_remove_at",
            arity: 2,
            returns: false,
        },
        Builtin::RemoveKey => Op {
            symbol: "sr_remove_key",
            arity: 2,
            returns: false,
        },
        Builtin::Sub => value("sr_sub", 2),
        Builtin::Mul => value("sr_mul", 2),
        Builtin::Div => value("sr_div", 2),
        Builtin::Rem => value("sr_rem", 2),
        Builtin::Quot => value("sr_quot", 2),
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
declare void @sr_str(ptr, ptr, i64)
declare void @sr_str_kept(ptr, ptr, ptr, i64)
declare i8 @sr_truthy(ptr)
declare void @sr_truthy_value(ptr, ptr)
declare void @sr_table_new(ptr)
declare void @sr_table_push(ptr, ptr)
declare i64 @sr_table_len(ptr)
declare i64 @sr_each_len(ptr)
declare void @sr_index_set(ptr, ptr, ptr)
declare void @sr_table_get(ptr, ptr, i64)
declare void @sr_table_put(ptr, ptr, i64, ptr)
declare void @sr_push(ptr, ptr)
declare void @sr_remove_at(ptr, ptr)
declare void @sr_remove_key(ptr, ptr)
declare void @sr_template(ptr, ptr, i64)
declare void @sr_field_get(ptr, ptr, ptr, i64, ptr)
declare void @sr_pick_get(ptr, ptr, ptr, ptr)
declare void @sr_pick_set(ptr, ptr, ptr)
declare void @sr_field_set(ptr, ptr, i64, ptr)
declare void @sr_index(ptr, ptr, ptr)
declare void @sr_range(ptr, ptr, ptr, ptr)
declare i8 @sr_name_is(ptr, i64, ptr, i64)
declare void @sr_print(ptr)
declare void @sr_print_parts(ptr, i64)
declare void @sr_add(ptr, ptr, ptr)
declare void @sr_append(ptr, ptr)
declare void @sr_sub(ptr, ptr, ptr)
declare void @sr_mul(ptr, ptr, ptr)
declare void @sr_div(ptr, ptr, ptr)
declare void @sr_rem(ptr, ptr, ptr)
declare void @sr_quot(ptr, ptr, ptr)
declare i64 @sr_quot_int(i64, i64)
declare void @sr_overflow(ptr, i64)
declare {i64, i1} @llvm.sadd.with.overflow.i64(i64, i64)
declare {i64, i1} @llvm.ssub.with.overflow.i64(i64, i64)
declare {i64, i1} @llvm.smul.with.overflow.i64(i64, i64)
declare i64 @sr_rem_int(i64, i64)
declare double @sr_rem_real(double, double)
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
declare void @sr_roots(ptr, i64)
declare void @sr_frame_push(ptr, i64)
declare void @sr_frame_pop()
declare void @sr_gc_point()
declare void @sr_bad_step()
declare void @sr_stack_base()
declare void @sr_stack_check()
declare void @sr_sources(ptr, i64, i8)
@SR_POS = external global i64
@SR_FRAMES = external global [1024 x ptr]
@SR_AT = external global [1024 x i64]
@SR_DEPTH = external global i32
";

mod call;
mod stmt;
mod value;

impl<'a> Emitter<'a> {
    fn new(program: &'a Program, types: Types, reuse: crate::reuse::Reuse) -> Self {
        Emitter {
            program,
            types,
            reuse,
            strings: HashMap::new(),
            constants: String::new(),
            functions: String::new(),
            body: String::new(),
            allocas: Vec::new(),
            roots: Vec::new(),
            temps: 0,
            labels: 0,
            loops: Vec::new(),
            marked: None,
            sites: String::new(),
            site_count: 0,
            cached: std::collections::HashSet::new(),
            mutable_str: false,
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
        self.roots.push(name.clone());
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

    // 제자리 잇기가 닿는 자리면 캐시를 쓰지 않는다 (그 상자를 고쳐 쓰므로).
    fn cached_str(&mut self, constant: &str) -> Option<String> {
        if self.mutable_str {
            return None;
        }
        let name = format!("@.v{}", &constant[3..]);
        if self.cached.insert(name.clone()) {
            let _ = writeln!(
                self.constants,
                "{name} = internal global %Value zeroinitializer"
            );
        }
        Some(name)
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

    // 16바이트 옮기기는 함수를 부를 일이 아니다.
    fn move_value(&mut self, dst: &str, src: &str) {
        let held = self.temp();
        self.line(&format!("{held} = load %Value, ptr {src}, align 8"));
        self.line(&format!("store %Value {held}, ptr {dst}, align 8"));
    }

    fn clear_value(&mut self, dst: &str) {
        self.line(&format!("store %Value zeroinitializer, ptr {dst}, align 8"));
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

// 프레임을 안 세운 함수에서는 빼는 줄도 지운다.
fn drop_frame(body: String, framed: bool) -> String {
    if framed {
        return body;
    }
    body.replace("  call void @sr_frame_pop()\n", "")
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
        self.roots.clear();
        self.temps = 0;
        self.module = module;
        self.marked = None;
        self.loops.clear();
    }

    // 값 자리는 모두 뿌리다. 등록하기 전에 비워 둬야 수집기가 쓰레기를 안 읽는다.
    // 뿌리가 없으면 프레임도 안전 지점도 값이 없다 — 부르는 쪽·안쪽이 대신 잰다.
    fn open_frame(&mut self) -> bool {
        let count = self.roots.len();
        if count == 0 {
            return false;
        }
        let held = count.max(1);
        let _ = writeln!(self.functions, "  %roots = alloca [{held} x ptr], align 8");
        for (at, name) in self.roots.iter().enumerate() {
            let _ = writeln!(
                self.functions,
                "  store %Value zeroinitializer, ptr {name}, align 8\n                   %root{at} = getelementptr inbounds [{held} x ptr], ptr %roots, i64 0, i64 {at}\n                   store ptr {name}, ptr %root{at}, align 8"
            );
        }
        let _ = writeln!(
            self.functions,
            "  call void @sr_frame_push(ptr %roots, i64 {count})\n  call void @sr_gc_point()"
        );
        true
    }

    fn close(&mut self, header: &str, prologue: &str, ret: Repr) {
        let _ = writeln!(self.functions, "{header} {{\nentry:");
        for line in &self.allocas {
            let _ = writeln!(self.functions, "{line}");
        }
        let framed = self.open_frame();
        self.functions.push_str(prologue);
        let body = std::mem::take(&mut self.body);
        self.functions.push_str(&drop_frame(body, framed));
        let tail = match ret {
            Repr::Boxed => "  ret void".to_string(),
            other => format!("  ret {} {}", other.llvm(), other.zero()),
        };
        if framed {
            let _ = writeln!(self.functions, "  call void @sr_frame_pop()");
        }
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
                self.move_value("%out", &held);
            }
            if let Some(depth) = depth {
                self.line(&format!("store i32 {depth}, ptr @SR_DEPTH, align 4"));
            }
            self.line("call void @sr_frame_pop()");
            self.line("ret i8 1");
            self.mark(&miss);
        }
        self.line("call void @sr_frame_pop()");
        self.line("ret i8 0");
        let header = format!(
            "define internal i8 @nouns_{module}(ptr %out, ptr %owner, ptr %name, i64 %len)"
        );
        let _ = writeln!(self.functions, "{header} {{\nentry:");
        let allocas = self.allocas.join("\n");
        if !allocas.is_empty() {
            let _ = writeln!(self.functions, "{allocas}");
        }
        let framed = self.open_frame();
        let body = std::mem::take(&mut self.body);
        self.functions.push_str(&drop_frame(body, framed));
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
                self.roots.push(format!("%l{slot}"));
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
            let _ = writeln!(prologue, "  store %Value zeroinitializer, ptr %out, align 8");
        }
        // 잎 함수는 자기를 다시 못 부르니 검사도 필요 없다.
        if calls_user(&function.body) {
            let _ = writeln!(prologue, "  call void @sr_stack_check()");
        }
        for (index, slot) in function.params.iter().enumerate() {
            let repr = params[index];
            if repr == Repr::Boxed {
                let _ = writeln!(
                    prologue,
                    "  %arg{index} = load %Value, ptr %p{index}, align 8\n  \
                     store %Value %arg{index}, ptr %l{slot}, align 8"
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
        // 상자에 담긴 전역과 리터럴 곳간이 붙박이 뿌리다.
        let mut fixed: Vec<String> = (0..self.program.globals)
            .filter(|&slot| Repr::of(self.types.globals[slot as usize]) == Repr::Boxed)
            .map(|slot| {
                format!(
                    "ptr getelementptr inbounds ([{} x %Value], ptr @globals, i64 0, i64 {slot})",
                    self.program.globals.max(1)
                )
            })
            .collect();
        let mut cached: Vec<String> = self.cached.iter().cloned().collect();
        cached.sort();
        fixed.extend(cached.into_iter().map(|name| format!("ptr {name}")));
        let held = fixed.len();
        let _ = writeln!(
            out,
            "@fixed = internal constant [{} x ptr] [{}]",
            held.max(1),
            if fixed.is_empty() {
                "ptr null".to_string()
            } else {
                fixed.join(", ")
            }
        );
        let _ = writeln!(out, "define i32 @main() {{\nentry:");
        let _ = writeln!(out, "  call void @sr_stack_base()");
        let _ = writeln!(
            out,
            "  call void @sr_sources(ptr @sources, i64 {count}, i8 {traced})"
        );
        let _ = writeln!(out, "  call void @sr_roots(ptr @fixed, i64 {held})");
        for id in &self.program.order {
            let _ = writeln!(out, "  call void @mod_{id}()");
        }
        let _ = writeln!(out, "  call void @sr_finish()");
        let _ = writeln!(out, "  ret i32 0\n}}");
        out
    }
}

fn calls_user(body: &[Stmt]) -> bool {
    fn in_expr(expr: &Expr) -> bool {
        match expr {
            Expr::Call { callee, args, .. } => {
                matches!(callee, Callee::User(_)) || args.iter().any(in_expr)
            }
            Expr::Field { owner, .. } => in_expr(owner),
            Expr::Index { owner, place, .. } => in_expr(owner) || in_expr(place),
            Expr::Pick { owner, key, .. } => in_expr(owner) || in_expr(key),
            Expr::Template(items) => items.iter().any(in_expr),
            Expr::Table(items, entries) => {
                items.iter().any(in_expr) || entries.iter().any(|(_, value)| in_expr(value))
            }
            Expr::Not(inner) | Expr::Ask { value: inner, .. } => in_expr(inner),
            Expr::And(left, right) | Expr::Or(left, right) => in_expr(left) || in_expr(right),
            _ => false,
        }
    }
    fn in_stmt(statement: &Stmt) -> bool {
        let here = match statement {
            Stmt::Set { value, .. } | Stmt::Eval(value) | Stmt::Return { value, .. } => {
                in_expr(value)
            }
            Stmt::SetField { owner, value, .. } => in_expr(owner) || in_expr(value),
            Stmt::SetPick {
                owner, key, value, ..
            } => in_expr(owner) || in_expr(key) || in_expr(value),
            Stmt::SetAt {
                owner,
                place,
                value,
                ..
            } => in_expr(owner) || in_expr(place) || in_expr(value),
            Stmt::Each { over, .. } => in_expr(over),
            Stmt::While { test, .. } => in_expr(test),
            Stmt::If { branches, .. } => branches.iter().any(|(test, _)| in_expr(test)),
            Stmt::Range {
                start, stop, step, ..
            } => in_expr(start) || in_expr(stop) || step.as_ref().is_some_and(in_expr),
            Stmt::Break | Stmt::Continue => false,
        };
        here || blocks(statement).into_iter().any(calls_user)
    }
    body.iter().any(in_stmt)
}



