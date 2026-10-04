//! 셈과 호출.

use super::*;

impl<'a> Emitter<'a> {
    // i64 산술은 조용히 감기지 않는다. 넘치면 그 자리에서 멈춘다.
    pub(super) fn overflowed(
        &mut self,
        intrinsic: &str,
        verb: &str,
        a: &str,
        b: &str,
        span: Span,
    ) -> String {
        let pair = self.temp();
        self.line(&format!(
            "{pair} = call {{i64, i1}} @llvm.{intrinsic}.with.overflow.i64(i64 {a}, i64 {b})"
        ));
        let out = self.temp();
        self.line(&format!("{out} = extractvalue {{i64, i1}} {pair}, 0"));
        let over = self.temp();
        self.line(&format!("{over} = extractvalue {{i64, i1}} {pair}, 1"));
        let trap = self.label("overflow");
        let ok = self.label("inbounds");
        self.line(&format!("br i1 {over}, label %{trap}, label %{ok}"));
        self.mark(&trap);
        self.at(span);
        let (name, len) = self.constant(verb);
        self.line(&format!("call void @sr_overflow(ptr {name}, i64 {len})"));
        self.line(&format!("br label %{ok}"));
        self.mark(&ok);
        out
    }

    fn nonzero(&mut self, divisor: &str, span: Span) {
        let zero = self.temp();
        self.line(&format!("{zero} = fcmp oeq double {divisor}, 0.0"));
        let trap = self.label("divzero");
        let ok = self.label("divok");
        self.line(&format!("br i1 {zero}, label %{trap}, label %{ok}"));
        self.mark(&trap);
        self.at(span);
        self.line("call void @sr_div_zero()");
        self.line(&format!("br label %{ok}"));
        self.mark(&ok);
    }

    fn finite(&mut self, made: &str, verb: &str, span: Span) {
        let size = self.temp();
        self.line(&format!(
            "{size} = call double @llvm.fabs.f64(double {made})"
        ));
        let over = self.temp();
        self.line(&format!(
            "{over} = fcmp oeq double {size}, 0x7FF0000000000000"
        ));
        let trap = self.label("overflow");
        let ok = self.label("inbounds");
        self.line(&format!("br i1 {over}, label %{trap}, label %{ok}"));
        self.mark(&trap);
        self.at(span);
        let (name, len) = self.constant(verb);
        self.line(&format!("call void @sr_overflow(ptr {name}, i64 {len})"));
        self.line(&format!("br label %{ok}"));
        self.mark(&ok);
    }

    fn negated(&mut self, value: &'a Expr, span: Span) -> Option<Val> {
        let (repr, name) = match self.type_of(value) {
            Ty::Int => {
                let found = self.value(value, Repr::Word);
                (Repr::Word, self.overflowed("ssub", "-", "0", &found, span))
            }
            Ty::Float => {
                let found = self.value(value, Repr::Real);
                let out = self.temp();
                self.line(&format!("{out} = fneg double {found}"));
                (Repr::Real, out)
            }
            _ => return None,
        };
        Some(Val { repr, name })
    }

    pub(super) fn inline_op(
        &mut self,
        op: Builtin,
        args: &'a [Expr],
        span: Span,
    ) -> Option<Val> {
        if let (Builtin::Neg, [value]) = (op, args) {
            return self.negated(value, span);
        }
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

        let direct = match op {
            Builtin::Rem if whole => Some((Repr::Word, "sr_rem_int")),
            Builtin::Rem if real => Some((Repr::Real, "sr_rem_real")),
            Builtin::Quot if whole => Some((Repr::Word, "sr_quot_int")),
            _ => None,
        };
        if let Some((kind, symbol)) = direct {
            let a = self.value(&args[0], kind);
            let b = self.value(&args[1], kind);
            let out = self.temp();
            let ty = kind.llvm();
            self.line(&format!("{out} = call {ty} @{symbol}({ty} {a}, {ty} {b})"));
            return Some(Val {
                repr: kind,
                name: out,
            });
        }
        // 나누다는 늘 실수, 몫은 정수끼리가 아니면 내림한 실수.
        if matches!(op, Builtin::Div | Builtin::Quot) && (whole || real) {
            let a = self.value(&args[0], Repr::Real);
            let b = self.value(&args[1], Repr::Real);
            self.nonzero(&b, span);
            let mut out = self.temp();
            self.line(&format!("{out} = fdiv double {a}, {b}"));
            self.finite(&out, "나누다", span);
            if op == Builtin::Quot {
                let floor = self.temp();
                self.line(&format!(
                    "{floor} = call double @llvm.floor.f64(double {out})"
                ));
                out = floor;
            }
            return Some(Val {
                repr: Repr::Real,
                name: out,
            });
        }
        let found = match op {
            Builtin::Add => Some(("sadd", "더하다", "fadd")),
            Builtin::Sub => Some(("ssub", "빼다", "fsub")),
            Builtin::Mul => Some(("smul", "곱하다", "fmul")),
            _ => None,
        };
        if let Some((intrinsic, verb, real_op)) = found {
            if whole {
                let a = self.value(&args[0], Repr::Word);
                let b = self.value(&args[1], Repr::Word);
                let out = self.overflowed(intrinsic, verb, &a, &b, span);
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
                self.finite(&out, verb, span);
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
        // 정수와 실수는 값 그대로 견준다. 정수를 실수로 바꾸면 2^53 위에서 뭉개진다.
        if real && (left == Ty::Int || right == Ty::Int) {
            let flip = right == Ty::Int;
            let reprs = if flip {
                [Repr::Real, Repr::Word]
            } else {
                [Repr::Word, Repr::Real]
            };
            let a = self.value(&args[0], reprs[0]);
            let b = self.value(&args[1], reprs[1]);
            let (whole, other) = if flip { (b, a) } else { (a, b) };
            let order = self.temp();
            self.line(&format!(
                "{order} = call i32 @sr_order_int_real(i64 {whole}, double {other})"
            ));
            let test = match (op, flip) {
                (Builtin::Equal, _) => "eq",
                (Builtin::Greater, false) | (Builtin::Less, true) => "sgt",
                _ => "slt",
            };
            let out = self.temp();
            self.line(&format!("{out} = icmp {test} i32 {order}, 0"));
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

    pub(super) fn call(&mut self, callee: Callee, args: &'a [Expr], span: Span) -> Val {
        if let Callee::Op(op) = callee {
            if let Some(found) = self.inline_op(op, args, span) {
                return found;
            }
        }
        match callee {
            Callee::Op(op) => self.op_call(op, args, span),
            Callee::User(func) => self.user_call(func, args, span),
        }
    }

    // `"..."을 출력한다` 는 새 글을 짓지 않고 바로 내보낸다. 가장 흔한 문장이다.
    fn print_parts(&mut self, args: &'a [Expr], span: Span) -> Option<Val> {
        let [Expr::Template(parts)] = args else {
            return None;
        };
        let array = self.template_array(parts);
        self.at(span);
        self.line(&format!(
            "call void @sr_print_parts(ptr {array}, i64 {})",
            parts.len()
        ));
        let out = self.nothing();
        Some(Val {
            repr: Repr::Boxed,
            name: out,
        })
    }

    pub(super) fn op_call(&mut self, op: Builtin, args: &'a [Expr], span: Span) -> Val {
        if op == Builtin::Print {
            if let Some(found) = self.print_parts(args, span) {
                return found;
            }
        }
        if op == Builtin::SortBy {
            return self.sort_by(args, span);
        }
        let mut given = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.expr(arg);
            given.push(self.boxed(value));
        }
        self.at(span);
        let found = operation(op);
        let out = if found.returns {
            self.slot()
        } else {
            self.nothing()
        };
        if found.symbol.is_empty() {
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

    pub(super) fn user_call(&mut self, func: FuncId, args: &'a [Expr], span: Span) -> Val {
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
        self.returned();
        self.guard_result(func, &made, span);
        made
    }

    pub(super) fn guard_result(&mut self, func: FuncId, made: &Val, span: Span) {
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
