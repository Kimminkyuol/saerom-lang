//! 문장과 흐름.

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn statement(&mut self, statement: &'a Stmt) {
        match statement {
            Stmt::SetPick {
                owner,
                key,
                value,
                span,
            } => {
                let owner = self.expr(owner);
                let owner = self.boxed(owner);
                let key = self.expr(key);
                let key = self.boxed(key);
                let value = self.expr(value);
                let value = self.boxed(value);
                self.at(*span);
                self.line(&format!(
                    "call void @sr_pick_set(ptr {owner}, ptr {key}, ptr {value})"
                ));
            }
            Stmt::Set { place, value } => {
                if self.append_in_place(*place, value) {
                    return;
                }
                // 제자리 잇기가 닿는 자리에 들어가는 리터럴만 캐시를 피한다.
                self.mutable_str =
                    self.reuse.allows(self.current, *place) && matches!(value, Expr::Str(_));
                let made = self.expr(value);
                self.mutable_str = false;
                self.write_place(*place, made);
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
            Stmt::SetAt {
                owner,
                place,
                value,
                span,
            } => {
                let owner = self.expr(owner);
                let owner = self.boxed(owner);
                let place = self.expr(place);
                let place = self.boxed(place);
                let value = self.expr(value);
                let value = self.boxed(value);
                self.at(*span);
                self.line(&format!(
                    "call void @sr_index_set(ptr {owner}, ptr {place}, ptr {value})"
                ));
            }
            Stmt::Each {
                place,
                over,
                body,
                span,
            } => self.each(*place, over, body, *span),
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
                        self.move_value("%out", &held);
                        self.line("call void @sr_frame_pop()");
                        self.line("ret void");
                    }
                    Some(ret) => {
                        let found = self.unboxed(value, ret);
                        self.line("call void @sr_frame_pop()");
                        self.line(&format!("ret {} {found}", ret.llvm()));
                    }
                }
                let next = self.label("dead");
                self.mark(&next);
            }
        }
    }

    pub(super) fn if_chain(&mut self, branches: &'a [(Expr, Vec<Stmt>)], otherwise: Option<&'a [Stmt]>) {
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

    pub(super) fn while_loop(&mut self, test: &'a Expr, body: &'a [Stmt]) {
        let head = self.label("while");
        let inside = self.label("do");
        let end = self.label("done");
        self.line(&format!("br label %{head}"));
        self.mark(&head);
        self.line("call void @sr_gc_point()");
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

    pub(super) fn range(
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
            None => self.boxed(Val {
                repr: Repr::Word,
                name: "1".into(),
            }),
        };
        let list = self.slot();
        self.at(span);
        self.line(&format!(
            "call void @sr_range(ptr {list}, ptr {start}, ptr {stop}, ptr {step})"
        ));
        let count = self.temp();
        self.line(&format!("{count} = call i64 @sr_table_len(ptr {list})"));
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
            "call void @sr_table_get(ptr {item}, ptr {list}, i64 {now})"
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
        self.line("call void @sr_gc_point()");
        let seen = self.temp();
        self.line(&format!("{seen} = load i64, ptr {index}, align 8"));
        let bumped = self.temp();
        self.line(&format!("{bumped} = add i64 {seen}, 1"));
        self.line(&format!("store i64 {bumped}, ptr {index}, align 8"));
        self.line(&format!("br label %{head}"));
        self.mark(&end);
    }

    pub(super) fn each(&mut self, place: Place, over: &'a Expr, body: &'a [Stmt], span: Span) {
        let held = self.expr(over);
        let list = self.boxed(held);
        self.at(span);
        let count = self.temp();
        self.line(&format!("{count} = call i64 @sr_each_len(ptr {list})"));
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
            "call void @sr_table_get(ptr {item}, ptr {list}, i64 {now})"
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
        self.line("call void @sr_gc_point()");
        let seen = self.temp();
        self.line(&format!("{seen} = load i64, ptr {index}, align 8"));
        let bumped = self.temp();
        self.line(&format!("{bumped} = add i64 {seen}, 1"));
        self.line(&format!("store i64 {bumped}, ptr {index}, align 8"));
        self.line(&format!("br label %{head}"));
        self.mark(&end);
    }

    pub(super) fn counted(
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
        // 방향은 간격의 부호가 정한다.
        let idle = self.temp();
        self.line(&format!("{idle} = icmp eq i64 {by}, 0"));
        let stop_here = self.label("badstep");
        let go = self.label("step");
        self.line(&format!("br i1 {idle}, label %{stop_here}, label %{go}"));
        self.mark(&stop_here);
        self.line("call void @sr_bad_step()");
        self.line(&format!("br label %{go}"));
        self.mark(&go);
        let back = self.temp();
        self.line(&format!("{back} = icmp slt i64 {by}, 0"));
        let delta = by.clone();

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
        self.line("call void @sr_gc_point()");
        let seen = self.temp();
        self.line(&format!("{seen} = load i64, ptr {counter}, align 8"));
        let bumped = self.temp();
        self.line(&format!("{bumped} = add i64 {seen}, {delta}"));
        self.line(&format!("store i64 {bumped}, ptr {counter}, align 8"));
        self.line(&format!("br label %{head}"));
        self.mark(&end);
    }
}
