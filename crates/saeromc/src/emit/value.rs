//! 값과 자리.

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn local_repr(&self, slot: LocalId) -> Repr {
        Repr::of(self.types.place(self.current, Place::Local(slot)))
    }

    pub(super) fn global_ptr(&mut self, slot: GlobalId) -> String {
        let name = self.temp();
        let count = self.program.globals.max(1);
        self.line(&format!(
            "{name} = getelementptr inbounds [{count} x %Value], ptr @globals, i64 0, i64 {slot}"
        ));
        name
    }

    pub(super) fn read_place(&mut self, place: Place) -> Val {
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

    pub(super) fn unboxed_from(&mut self, holder: &str, repr: Repr) -> Val {
        let boxed = Val {
            repr: Repr::Boxed,
            name: holder.to_string(),
        };
        let name = self.unboxed(boxed, repr);
        Val { repr, name }
    }

    pub(super) fn append_in_place(&mut self, place: Place, value: &'a Expr) -> bool {
        let Expr::Call {
            callee: Callee::Op(Builtin::Add),
            args,
            span,
        } = value
        else {
            return false;
        };
        if args.len() != 2 || !self.reuse.allows(self.current, place) {
            return false;
        }
        let same = match (&args[0], place) {
            (Expr::Local(slot), Place::Local(kept)) => *slot == kept,
            (Expr::Global(slot), Place::Global(kept)) => *slot == kept,
            _ => false,
        };
        if !same {
            return false;
        }
        let holder = self.place_ptr(place);
        let tail = self.expr(&args[1]);
        let tail = self.boxed(tail);
        self.at(*span);
        self.line(&format!("call void @sr_append(ptr {holder}, ptr {tail})"));
        true
    }

    pub(super) fn place_ptr(&mut self, place: Place) -> String {
        match place {
            Place::Local(slot) => format!("%l{slot}"),
            Place::Global(slot) => self.global_ptr(slot),
        }
    }

    pub(super) fn write_place(&mut self, place: Place, val: Val) {
        match place {
            Place::Local(slot) => {
                let repr = self.local_repr(slot);
                if repr == Repr::Boxed {
                    let source = self.boxed(val);
                    self.move_value(&format!("%l{slot}"), &source);
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
                    self.move_value(&holder, &source);
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

    pub(super) fn value(&mut self, expr: &'a Expr, want: Repr) -> String {
        let found = self.expr(expr);
        self.unboxed(found, want)
    }

    pub(super) fn signature(&self, id: FuncId) -> (Repr, Vec<Repr>) {
        let found = &self.program.functions[id as usize];
        let ret = Repr::of(self.types.returns[id as usize]);
        let params = found
            .params
            .iter()
            .map(|&slot| Repr::of(self.types.locals[id as usize][slot as usize]))
            .collect();
        (ret, params)
    }

    pub(super) fn template_array(&mut self, parts: &'a [Expr]) -> String {
        let count = parts.len();
        let array = self.raw(&format!("[{count} x %Value]"));
        for (index, part) in parts.iter().enumerate() {
            let value = self.expr(part);
            let held = self.boxed(value);
            let cell = self.temp();
            self.line(&format!(
                "{cell} = getelementptr inbounds [{count} x %Value], ptr {array}, i64 0, i64 {index}"
            ));
            self.move_value(&cell, &held);
        }
        array
    }

    pub(super) fn expr(&mut self, expr: &'a Expr) -> Val {
        match expr {
            Expr::Pick { owner, key, span } => {
                let owner = self.expr(owner);
                let owner = self.boxed(owner);
                let key = self.expr(key);
                let key = self.boxed(key);
                let nouns = self.nouns_argument(self.module);
                let out = self.slot();
                self.at(*span);
                self.line(&format!(
                    "call void @sr_pick_get(ptr {out}, ptr {owner}, ptr {key}, {nouns})"
                ));
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
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
                self.clear_value(&out);
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Str(found) => {
                let out = self.slot();
                let (name, len) = self.constant(found);
                match self.cached_str(&name) {
                    Some(cache) => self.line(&format!(
                        "call void @sr_str_kept(ptr {out}, ptr {cache}, ptr {name}, i64 {len})"
                    )),
                    None => self.line(&format!(
                        "call void @sr_str(ptr {out}, ptr {name}, i64 {len})"
                    )),
                }
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Local(slot) => self.read_place(Place::Local(*slot)),
            Expr::Global(slot) => self.read_place(Place::Global(*slot)),
            Expr::Table(items, entries) => {
                let out = self.slot();
                self.line(&format!("call void @sr_table_new(ptr {out})"));
                for item in items {
                    let value = self.expr(item);
                    let held = self.boxed(value);
                    self.line(&format!("call void @sr_table_push(ptr {out}, ptr {held})"));
                }
                for (key, value) in entries {
                    let value = self.expr(value);
                    let held = self.boxed(value);
                    let (name, len) = self.name_of(*key);
                    self.line(&format!(
                        "call void @sr_table_put(ptr {out}, ptr {name}, i64 {len}, ptr {held})"
                    ));
                }
                Val {
                    repr: Repr::Boxed,
                    name: out,
                }
            }
            Expr::Template(parts) => {
                let count = parts.len();
                let array = self.template_array(parts);
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

    pub(super) fn direct_noun(
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

    pub(super) fn name_of(&mut self, field: crate::intern::Symbol) -> (String, usize) {
        let text = self.program.names.name(field).to_string();
        self.constant(&text)
    }

    pub(super) fn shortcut(&mut self, left: &'a Expr, right: &'a Expr, all: bool) -> Val {
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
