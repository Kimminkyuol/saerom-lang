use crate::builtins::Builtin;
use crate::diag::Diag;
use crate::hir::*;
use std::collections::HashMap;
use std::fmt::Write;

pub fn emit(program: &Program, triple: &str) -> Result<String, Vec<Diag>> {
    let mut emitter = Emitter::new(program);
    emitter.run();
    if !emitter.errors.is_empty() {
        return Err(emitter.errors);
    }
    Ok(emitter.finish(triple))
}

struct Emitter<'a> {
    program: &'a Program,
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
        Builtin::Convert => value("sr_convert", 2),
        Builtin::Join => value("sr_join", 1),
        Builtin::JoinWith => value("sr_join_with", 2),
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
        Builtin::StartsWith => value("sr_starts", 2),
        Builtin::Contains => value("sr_contains", 2),
        Builtin::EndsWith => value("sr_ends", 2),
        Builtin::Trim => value("sr_trim", 1),
        Builtin::Split => value("sr_split", 2),
        Builtin::Read => value("sr_read", 1),
        Builtin::ReadLine => value("sr_readline", 0),
        Builtin::Open => value("sr_open", 1),
        Builtin::Write => Op {
            symbol: "sr_write",
            arity: 2,
            returns: false,
        },
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
declare void @sr_join(ptr, ptr)
declare void @sr_join_with(ptr, ptr, ptr)
declare void @sr_split(ptr, ptr, ptr)
declare void @sr_contains(ptr, ptr, ptr)
declare void @sr_starts(ptr, ptr, ptr)
declare void @sr_ends(ptr, ptr, ptr)
declare void @sr_trim(ptr, ptr)
declare void @sr_check_bool(ptr, ptr, i64)
declare void @sr_check_value(ptr, ptr, i64)
declare void @sr_read(ptr, ptr)
declare void @sr_readline(ptr)
declare void @sr_open(ptr, ptr)
declare void @sr_write(ptr, ptr)
declare void @sr_close(ptr)
declare void @sr_stop(ptr)
declare void @sr_sources(ptr, i64)
@SR_POS = external global i64
@SR_FRAMES = external global [1024 x ptr]
@SR_AT = external global [1024 x i64]
@SR_DEPTH = external global i32
";

impl<'a> Emitter<'a> {
    fn new(program: &'a Program) -> Self {
        Emitter {
            program,
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

    fn enter(&mut self, name: &str, at: Option<crate::diag::Span>) -> String {
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
        depth
    }

    fn packed(&self, span: crate::diag::Span) -> u64 {
        let width = span.end.saturating_sub(span.col).min(0xFFF);
        ((self.module as u64) << 48)
            | ((span.line as u64 & 0xFF_FFFF) << 24)
            | ((span.col as u64 & 0xFFF) << 12)
            | width as u64
    }

    fn at(&mut self, span: crate::diag::Span) {
        let packed = self.packed(span);
        if self.marked == Some(packed) {
            return;
        }
        self.marked = Some(packed);
        self.line(&format!("store i64 {packed}, ptr @SR_POS, align 8"));
    }

    fn label(&mut self, tag: &str) -> String {
        self.labels += 1;
        format!("{tag}{}", self.labels)
    }

    fn slot(&mut self) -> String {
        self.temps += 1;
        let name = format!("%v{}", self.temps);
        self.allocas
            .push(format!("  {name} = alloca %Value, align 8"));
        name
    }

    fn raw(&mut self, kind: &str) -> String {
        self.temps += 1;
        let name = format!("%v{}", self.temps);
        self.allocas
            .push(format!("  {name} = alloca {kind}, align 8"));
        name
    }

    fn temp(&mut self) -> String {
        self.temps += 1;
        format!("%v{}", self.temps)
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

    fn finish(&mut self, triple: &str) -> String {
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
        let _ = writeln!(out, "  call void @sr_sources(ptr @sources, i64 {count})");
        for id in &self.program.order {
            let _ = writeln!(out, "  call void @mod_{id}()");
        }
        let _ = writeln!(out, "  ret i32 0\n}}");
        out
    }

    fn open(&mut self, module: ModuleId) {
        self.body.clear();
        self.allocas.clear();
        self.temps = 0;
        self.module = module;
        self.marked = None;
        self.loops.clear();
    }

    fn close(&mut self, header: &str, prologue: &str) {
        let _ = writeln!(self.functions, "{header} {{\nentry:");
        for line in &self.allocas {
            let _ = writeln!(self.functions, "{line}");
        }
        self.functions.push_str(prologue);
        self.functions.push_str(&self.body);
        let _ = writeln!(self.functions, "  ret void\n}}\n");
    }

    fn nouns_argument(&self, module: ModuleId) -> String {
        if self.program.modules[module as usize].nouns.is_empty() {
            "ptr null".to_string()
        } else {
            format!("ptr @nouns_{module}")
        }
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
            self.line(&format!("call void @fn_{func}(ptr %out, ptr %owner)"));
            self.line(&format!("store i32 {depth}, ptr @SR_DEPTH, align 4"));
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
        let mut prologue = String::new();
        for slot in 0..function.locals {
            self.allocas
                .push(format!("  %l{slot} = alloca %Value, align 8"));
            let _ = writeln!(prologue, "  call void @sr_nothing(ptr %l{slot})");
        }
        let _ = writeln!(prologue, "  call void @sr_nothing(ptr %out)");
        for (index, slot) in function.params.iter().enumerate() {
            let _ = writeln!(
                prologue,
                "  call void @sr_copy(ptr %l{slot}, ptr %p{index})"
            );
        }
        for statement in &function.body {
            self.statement(statement);
        }
        let params: Vec<String> = (0..function.params.len())
            .map(|index| format!("ptr %p{index}"))
            .collect();
        let header = format!(
            "define internal void @fn_{id}(ptr %out{}{})",
            if params.is_empty() { "" } else { ", " },
            params.join(", ")
        );
        self.close(&header, &prologue);
    }

    fn module_init(&mut self, id: ModuleId) {
        self.open(id);
        let module = &self.program.modules[id as usize];
        for statement in &module.init {
            self.statement(statement);
        }
        let header = format!("define internal void @mod_{id}()");
        self.close(&header, "");
    }

    fn place(&mut self, place: Place) -> String {
        match place {
            Place::Local(slot) => format!("%l{slot}"),
            Place::Global(slot) => {
                let name = self.temp();
                let count = self.program.globals.max(1);
                self.line(&format!(
                    "{name} = getelementptr inbounds [{count} x %Value], ptr @globals, i64 0, i64 {slot}"
                ));
                name
            }
        }
    }

    fn truthy(&mut self, value: &str) -> String {
        let raw = self.temp();
        self.line(&format!("{raw} = call i8 @sr_truthy(ptr {value})"));
        let test = self.temp();
        self.line(&format!("{test} = icmp ne i8 {raw}, 0"));
        test
    }
}

impl<'a> Emitter<'a> {
    fn statement(&mut self, statement: &'a Stmt) {
        match statement {
            Stmt::Set { place, value } => {
                let value = self.expr(value);
                let place = self.place(*place);
                self.line(&format!("call void @sr_copy(ptr {place}, ptr {value})"));
            }
            Stmt::SetField {
                owner,
                field,
                value,
                ..
            } => {
                let owner = self.expr(owner);
                let value = self.expr(value);
                let (name, len) = self.name_of(*field);
                self.line(&format!(
                    "call void @sr_field_set(ptr {owner}, ptr {name}, i64 {len}, ptr {value})"
                ));
            }
            Stmt::With {
                call, place, body, ..
            } => self.with_block(call, *place, body),
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
                self.line(&format!("call void @sr_copy(ptr %out, ptr {value})"));
                self.line("ret void");
                let next = self.label("dead");
                self.mark(&next);
            }
        }
    }

    fn with_block(&mut self, call: &'a Expr, place: Place, body: &'a [Stmt]) {
        let opened = self.expr(call);
        let slot = self.place(place);
        self.line(&format!("call void @sr_copy(ptr {slot}, ptr {opened})"));
        for statement in body {
            self.statement(statement);
        }
        self.line(&format!("call void @sr_close(ptr {slot})"));
    }

    fn if_chain(&mut self, branches: &'a [(Expr, Vec<Stmt>)], otherwise: Option<&'a [Stmt]>) {
        let end = self.label("endif");
        for (test, body) in branches {
            let value = self.expr(test);
            let taken = self.truthy(&value);
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
        let taken = self.truthy(&value);
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
        span: crate::diag::Span,
    ) {
        let start = self.expr(start);
        let stop = self.expr(stop);
        let step = match step {
            Some(found) => self.expr(found),
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
        let target = self.place(place);
        self.line(&format!("call void @sr_copy(ptr {target}, ptr {item})"));
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

    fn name_of(&mut self, field: crate::intern::Symbol) -> (String, usize) {
        let text = self.program.names.name(field).to_string();
        self.constant(&text)
    }
}

impl<'a> Emitter<'a> {
    fn expr(&mut self, expr: &'a Expr) -> String {
        match expr {
            Expr::Local(slot) => format!("%l{slot}"),
            Expr::Global(_) => {
                let Expr::Global(slot) = expr else {
                    unreachable!()
                };
                self.place(Place::Global(*slot))
            }
            Expr::Int(found) => {
                let out = self.slot();
                self.line(&format!("call void @sr_int(ptr {out}, i64 {found})"));
                out
            }
            Expr::Float(found) => {
                let out = self.slot();
                let bits = format!("0x{:016X}", found.to_bits());
                self.line(&format!("call void @sr_float(ptr {out}, double {bits})"));
                out
            }
            Expr::Bool(found) => {
                let out = self.slot();
                self.line(&format!(
                    "call void @sr_bool(ptr {out}, i8 {})",
                    u8::from(*found)
                ));
                out
            }
            Expr::Nothing => {
                let out = self.slot();
                self.line(&format!("call void @sr_nothing(ptr {out})"));
                out
            }
            Expr::Str(found) => {
                let out = self.slot();
                let (name, len) = self.constant(found);
                self.line(&format!(
                    "call void @sr_str(ptr {out}, ptr {name}, i64 {len})"
                ));
                out
            }
            Expr::List(items) => {
                let out = self.slot();
                self.line(&format!("call void @sr_list_new(ptr {out})"));
                for item in items {
                    let value = self.expr(item);
                    self.line(&format!("call void @sr_list_push(ptr {out}, ptr {value})"));
                }
                out
            }
            Expr::Dict(entries) => {
                let out = self.slot();
                self.line(&format!("call void @sr_dict_new(ptr {out})"));
                for (key, value) in entries {
                    let value = self.expr(value);
                    let (name, len) = self.name_of(*key);
                    self.line(&format!(
                        "call void @sr_dict_put(ptr {out}, ptr {name}, i64 {len}, ptr {value})"
                    ));
                }
                out
            }
            Expr::Template(parts) => {
                let count = parts.len();
                let array = self.raw(&format!("[{count} x %Value]"));
                for (index, part) in parts.iter().enumerate() {
                    let value = self.expr(part);
                    let cell = self.temp();
                    self.line(&format!(
                        "{cell} = getelementptr inbounds [{count} x %Value], ptr {array}, i64 0, i64 {index}"
                    ));
                    self.line(&format!("call void @sr_copy(ptr {cell}, ptr {value})"));
                }
                let out = self.slot();
                self.line(&format!(
                    "call void @sr_template(ptr {out}, ptr {array}, i64 {count})"
                ));
                out
            }
            Expr::Field { owner, field, span } => {
                let owner = self.expr(owner);
                self.at(*span);
                let (name, len) = self.name_of(*field);
                let nouns = self.nouns_argument(self.module);
                let out = self.slot();
                self.line(&format!(
                    "call void @sr_field_get(ptr {out}, ptr {owner}, ptr {name}, i64 {len}, {nouns})"
                ));
                out
            }
            Expr::Index { owner, place, span } => {
                let owner = self.expr(owner);
                let place = self.expr(place);
                self.at(*span);
                let out = self.slot();
                self.line(&format!(
                    "call void @sr_index(ptr {out}, ptr {owner}, ptr {place})"
                ));
                out
            }
            Expr::Call { callee, args, span } => self.call(*callee, args, *span),
            Expr::Not(value) => {
                let value = self.expr(value);
                let out = self.slot();
                self.line(&format!("call void @sr_not(ptr {out}, ptr {value})"));
                out
            }
            Expr::Ask { value, verb, .. } => {
                let value = self.expr(value);
                let (name, len) = self.name_of(*verb);
                self.line(&format!(
                    "call void @sr_check_bool(ptr {value}, ptr {name}, i64 {len})"
                ));
                value
            }
            Expr::And(left, right) => self.shortcut(left, right, true),
            Expr::Or(left, right) => self.shortcut(left, right, false),
        }
    }

    fn shortcut(&mut self, left: &'a Expr, right: &'a Expr, all: bool) -> String {
        let out = self.slot();
        let value = self.expr(left);
        let taken = self.truthy(&value);
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
        let seen = self.truthy(&value);
        let bit = self.temp();
        self.line(&format!("{bit} = zext i1 {seen} to i8"));
        self.line(&format!("call void @sr_bool(ptr {out}, i8 {bit})"));
        self.line(&format!("br label %{end}"));
        self.mark(&quick.clone());
        self.line(&format!(
            "call void @sr_bool(ptr {out}, i8 {})",
            u8::from(!all)
        ));
        self.line(&format!("br label %{end}"));
        self.mark(&end);
        out
    }

    fn call(&mut self, callee: Callee, args: &'a [Expr], span: crate::diag::Span) -> String {
        let mut given = Vec::with_capacity(args.len());
        for arg in args {
            given.push(self.expr(arg));
        }
        self.at(span);
        match callee {
            Callee::Op(op) => {
                let found = operation(op);
                let out = self.slot();
                if found.symbol.is_empty() {
                    self.line(&format!("call void @sr_nothing(ptr {out})"));
                    return out;
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
                out
            }
            Callee::User(func) => {
                let out = self.slot();
                let mut passed = vec![format!("ptr {out}")];
                passed.extend(given.iter().map(|name| format!("ptr {name}")));
                let name = self.program.functions[func as usize].name.to_string();
                let depth = self.enter(&name, Some(span));
                self.line(&format!("call void @fn_{func}({})", passed.join(", ")));
                self.line(&format!("store i32 {depth}, ptr @SR_DEPTH, align 4"));
                let function = &self.program.functions[func as usize];
                let (kind, name) = (function.kind, function.name.to_string());
                match kind {
                    Kind::Predicate => {
                        let (text, len) = self.constant(&name);
                        self.line(&format!(
                            "call void @sr_check_bool(ptr {out}, ptr {text}, i64 {len})"
                        ));
                    }
                    Kind::Noun => {
                        let (text, len) = self.constant(&name);
                        self.line(&format!(
                            "call void @sr_check_value(ptr {out}, ptr {text}, i64 {len})"
                        ));
                    }
                    Kind::Verb => {}
                }
                out
            }
        }
    }
}
