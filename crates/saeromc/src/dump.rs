use crate::lex::{Num, Part, Tok, Token};

pub fn tokens(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| line(&token.tok))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn line(tok: &Tok) -> String {
    match tok {
        Tok::Name(name) => format!("name {name}"),
        Tok::Verb { name, pos, ending } => {
            format!("verb {name} {} {}", pos.as_str(), ending.as_str())
        }
        Tok::Copula { ending } => format!("copula {}", ending.as_str()),
        Tok::Particle { role, canon } => format!("particle {canon} {role}"),
        Tok::Keyword(word) => format!("keyword {word}"),
        Tok::Number(value) => format!("number {}", number(value)),
        Tok::Str(text) => format!("string {}", escape(text)),
        Tok::Template(parts) => format!("template {}", template(parts)),
        Tok::Symbol(ch) => format!("symbol {ch}"),
        Tok::Indent(depth) => format!("indent {depth}"),
        Tok::Dedent(depth) => format!("dedent {depth}"),
        Tok::Newline => "newline".into(),
        Tok::Eof => "eof".into(),
    }
}

fn number(value: &Num) -> String {
    match value {
        Num::Int(found) => found.to_string(),
        Num::Float(found) => format!("{found:?}"),
    }
}

fn template(parts: &[Part]) -> String {
    parts
        .iter()
        .map(|part| match part {
            Part::Text(text) => format!("text:{}", escape(text)),
            Part::Expr { source, .. } => format!("expr:{}", escape(source)),
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

use crate::ast::{Block, CallExpr, Expr, Literal, LoopKind, Slot, Stmt, Target, TemplatePart};
use crate::sig::Marker;

enum Tree {
    Leaf(String),
    Node(String, Vec<Tree>),
}

fn leaf(text: impl Into<String>) -> Tree {
    Tree::Leaf(text.into())
}

fn node(label: impl Into<String>, children: Vec<Tree>) -> Tree {
    Tree::Node(label.into(), children)
}

pub fn ast(statements: &[Stmt]) -> String {
    let mut out = String::new();
    for statement in statements {
        write(&statement_tree(statement), 0, &mut out);
    }
    out
}

fn write(tree: &Tree, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    match tree {
        Tree::Leaf(text) => out.push_str(&format!("{pad}{text}\n")),
        Tree::Node(label, children) => {
            out.push_str(&format!("{pad}({label}\n"));
            for child in children {
                write(child, depth + 1, out);
            }
            out.push_str(&format!("{pad})\n"));
        }
    }
}

fn marker_name(marker: Marker) -> &'static str {
    match marker {
        Marker::Case(particle) => particle,
        Marker::Bare => "-",
        Marker::Module => "모듈",
    }
}

fn slot_tree(slot: &Slot) -> Tree {
    node(
        format!("slot {}", marker_name(slot.marker)),
        vec![expr_tree(&slot.expr)],
    )
}

fn call_tree(call: &CallExpr) -> Tree {
    let head = format!(
        "call {} tail={} neg={} asks={}",
        call.verb,
        call.tail.as_deref().unwrap_or("-"),
        u8::from(call.negated),
        u8::from(call.asks)
    );
    node(head, call.slots.iter().map(slot_tree).collect())
}

fn joined(verb: &str, left: &Expr, right: &Expr) -> Tree {
    node(
        format!("call {verb} tail=- neg=0 asks=0"),
        vec![
            node("slot -", vec![expr_tree(left)]),
            node("slot -", vec![expr_tree(right)]),
        ],
    )
}

fn expr_tree(expr: &Expr) -> Tree {
    match expr {
        Expr::Pick { owner, key, .. } => node("pick", vec![expr_tree(owner), expr_tree(key)]),
        Expr::Spot { owner, place, .. } => {
            node("spot", vec![expr_tree(owner), expr_tree(place)])
        }
        Expr::Literal { value, .. } => leaf(match value {
            Literal::Nothing => "lit nothing".to_string(),
            Literal::Int(found) => format!("lit int {found}"),
            Literal::Float(found) => format!("lit float {found:?}"),
            Literal::Str(found) => format!("lit str {}", escape(found)),
            Literal::Bool(found) => format!("lit bool {}", if *found { "참" } else { "거짓" }),
        }),
        Expr::Name { name, .. } => leaf(format!("name {name}")),
        Expr::Table { items, entries, .. } => {
            node(
                "table",
                items
                    .iter()
                    .map(expr_tree)
                    .chain(entries.iter().map(|(key, value)| {
                        node(format!("entry {key}"), vec![expr_tree(value)])
                    }))
                    .collect(),
            )
        }
        Expr::Template { parts, .. } => node(
            "template",
            parts
                .iter()
                .map(|part| match part {
                    TemplatePart::Text(text) => leaf(format!("lit str {}", escape(text))),
                    TemplatePart::Expr(inner) => expr_tree(inner),
                })
                .collect(),
        ),
        Expr::Field { owner, name, .. } => {
            node(format!("field {name}"), vec![expr_tree(owner)])
        }
        Expr::Call(call) => call_tree(call),
        Expr::Passive(passive) => {
            let mut children = vec![node("head", vec![expr_tree(&passive.head)])];
            children.extend(passive.slots.iter().map(slot_tree));
            node(format!("passive {}", passive.verb), children)
        }
        Expr::And { left, right, .. } => joined("그리고", left, right),
        Expr::Or { left, right, .. } => joined("또는", left, right),
    }
}

fn target_tree(target: &Target) -> Tree {
    let mut tree = leaf(format!("name {}", target.root));
    for field in &target.fields {
        tree = match field {
            crate::ast::Selector::Name(name) => node(format!("field {name}"), vec![tree]),
            crate::ast::Selector::Pick(key) => node("pick", vec![tree, expr_tree(key)]),
            crate::ast::Selector::Spot(place) => node("spot", vec![tree, expr_tree(place)]),
        };
    }
    tree
}

fn block_tree(label: &str, body: &Block) -> Tree {
    node(label, body.iter().map(statement_tree).collect())
}

fn statement_tree(statement: &Stmt) -> Tree {
    match statement {
        Stmt::Declare { assigns, .. } => node(
            "declare",
            assigns
                .iter()
                .flat_map(|(target, value)| {
                    [
                        node("target", vec![target_tree(target)]),
                        node("value", vec![expr_tree(value)]),
                    ]
                })
                .collect(),
        ),
        Stmt::Exec { calls, .. } => node("exec", calls.iter().map(call_tree).collect()),
        Stmt::Value { expr, .. } => node("value", vec![expr_tree(expr)]),
        Stmt::If {
            branches,
            otherwise,
            ..
        } => {
            let mut children: Vec<Tree> = branches
                .iter()
                .map(|(test, body)| {
                    node(
                        "branch",
                        vec![
                            node("test", vec![expr_tree(test)]),
                            block_tree("body", body),
                        ],
                    )
                })
                .collect();
            if let Some(body) = otherwise {
                children.push(block_tree("otherwise", body));
            }
            node("if", children)
        }
        Stmt::Loop { kind, body, .. } => match kind {
            LoopKind::Each { variable, over } => node(
                format!("each {variable}"),
                vec![
                    node("over", vec![expr_tree(over)]),
                    block_tree("body", body),
                ],
            ),
            LoopKind::Range {
                variable,
                start,
                stop,
                step,
            } => {
                let mut children = vec![
                    node("start", vec![expr_tree(start)]),
                    node("stop", vec![expr_tree(stop)]),
                ];
                if let Some(step) = step {
                    children.push(node("step", vec![expr_tree(step)]));
                }
                children.push(block_tree("body", body));
                node(format!("loop range {variable}"), children)
            }
            LoopKind::While { test } => node(
                "loop while",
                vec![
                    node("test", vec![expr_tree(test)]),
                    block_tree("body", body),
                ],
            ),
        },
        Stmt::Break { .. } => leaf("break"),
        Stmt::Continue { .. } => leaf("continue"),
        Stmt::Return { value, .. } => node("return", vec![expr_tree(value)]),
        Stmt::Define {
            name, params, body, ..
        } => {
            let shown: Vec<String> = params
                .iter()
                .map(|(marker, name)| format!("{}:{name}", marker_name(*marker)))
                .collect();
            node(
                format!("define {name} [{}]", shown.join(" ")),
                vec![block_tree("body", body)],
            )
        }
        Stmt::Noun {
            name, owner, body, ..
        } => node(
            format!("noun {name} of {owner}"),
            vec![block_tree("body", body)],
        ),
        Stmt::Import { module, names, .. } => {
            let shown = names
                .as_ref()
                .map_or("*".to_string(), |names| names.join(" "));
            leaf(format!("import {module} [{shown}]"))
        }
    }
}

use crate::hir;

pub fn hir(program: &hir::Program) -> String {
    let mut out = String::new();
    out.push_str(&format!("globals {}\n", program.globals));
    for &id in &program.order {
        let module = &program.modules[id as usize];
        out.push_str(&format!("module {} {}\n", id, module.name));
        let mut nouns: Vec<String> = module
            .nouns
            .iter()
            .map(|(&field, &func)| format!("{}=#{func}", program.names.name(field)))
            .collect();
        nouns.sort();
        if !nouns.is_empty() {
            out.push_str(&format!("  nouns {}\n", nouns.join(" ")));
        }
        for statement in &module.init {
            write(&hir_stmt(program, statement), 1, &mut out);
        }
    }
    for (index, function) in program.functions.iter().enumerate() {
        let params: Vec<String> = function
            .params
            .iter()
            .map(|slot| slot.to_string())
            .collect();
        out.push_str(&format!(
            "fn #{index} {} {} module={} params=[{}] locals={}\n",
            function.name,
            match function.kind {
                hir::Kind::Verb => "verb",
                hir::Kind::Noun => "noun",
            },
            function.module,
            params.join(" "),
            function.locals
        ));
        for statement in &function.body {
            write(&hir_stmt(program, statement), 1, &mut out);
        }
    }
    out
}

fn place_name(place: hir::Place) -> String {
    match place {
        hir::Place::Local(slot) => format!("local {slot}"),
        hir::Place::Global(slot) => format!("global {slot}"),
    }
}

fn hir_block(program: &hir::Program, label: &str, body: &[hir::Stmt]) -> Tree {
    node(
        label,
        body.iter().map(|one| hir_stmt(program, one)).collect(),
    )
}

fn hir_stmt(program: &hir::Program, statement: &hir::Stmt) -> Tree {
    use hir::Stmt as S;
    match statement {
        S::SetAt {
            owner,
            place,
            value,
            ..
        } => node(
            "set at",
            vec![
                hir_expr(program, owner),
                hir_expr(program, place),
                hir_expr(program, value),
            ],
        ),
        S::Each {
            place, over, body, ..
        } => node(
            format!("each {}", place_name(*place)),
            vec![hir_expr(program, over), hir_block(program, "body", body)],
        ),
        S::SetPick {
            owner, key, value, ..
        } => node(
            "set pick",
            vec![
                hir_expr(program, owner),
                hir_expr(program, key),
                hir_expr(program, value),
            ],
        ),
        S::Set { place, value } => node(
            format!("set {}", place_name(*place)),
            vec![hir_expr(program, value)],
        ),
        S::SetField {
            owner,
            field,
            value,
            ..
        } => node(
            format!("setfield {}", program.names.name(*field)),
            vec![hir_expr(program, owner), hir_expr(program, value)],
        ),
        S::Eval(value) => node("eval", vec![hir_expr(program, value)]),
        S::If {
            branches,
            otherwise,
        } => {
            let mut children: Vec<Tree> = branches
                .iter()
                .map(|(test, body)| {
                    node(
                        "branch",
                        vec![
                            node("test", vec![hir_expr(program, test)]),
                            hir_block(program, "body", body),
                        ],
                    )
                })
                .collect();
            if let Some(body) = otherwise {
                children.push(hir_block(program, "otherwise", body));
            }
            node("if", children)
        }
        S::Range {
            place,
            start,
            stop,
            step,
            body,
            ..
        } => {
            let mut children = vec![
                node("start", vec![hir_expr(program, start)]),
                node("stop", vec![hir_expr(program, stop)]),
            ];
            if let Some(step) = step {
                children.push(node("step", vec![hir_expr(program, step)]));
            }
            children.push(hir_block(program, "body", body));
            node(format!("range {}", place_name(*place)), children)
        }
        S::While { test, body } => node(
            "while",
            vec![
                node("test", vec![hir_expr(program, test)]),
                hir_block(program, "body", body),
            ],
        ),
        S::Break => leaf("break"),
        S::Continue => leaf("continue"),
        S::Return { value, .. } => node("return", vec![hir_expr(program, value)]),
    }
}

fn hir_expr(program: &hir::Program, expr: &hir::Expr) -> Tree {
    use hir::Expr as E;
    match expr {
        E::Pick { owner, key, .. } => node(
            "pick",
            vec![hir_expr(program, owner), hir_expr(program, key)],
        ),
        E::Int(value) => leaf(format!("int {value}")),
        E::Float(value) => leaf(format!("float {value:?}")),
        E::Str(value) => leaf(format!("str {}", escape(value))),
        E::Bool(value) => leaf(format!("bool {}", if *value { "참" } else { "거짓" })),
        E::Nothing => leaf("nothing"),
        E::Local(slot) => leaf(format!("local {slot}")),
        E::Global(slot) => leaf(format!("global {slot}")),
        E::Table(items, entries) => node(
            "table",
            items
                .iter()
                .map(|i| hir_expr(program, i))
                .chain(entries.iter().map(|(key, value)| {
                    node(
                        format!("entry {}", program.names.name(*key)),
                        vec![hir_expr(program, value)],
                    )
                }))
                .collect(),
        ),
        E::Template(parts) => node(
            "template",
            parts.iter().map(|p| hir_expr(program, p)).collect(),
        ),
        E::Field { owner, field, .. } => node(
            format!("field {}", program.names.name(*field)),
            vec![hir_expr(program, owner)],
        ),
        E::Index { owner, place, .. } => node(
            "index",
            vec![hir_expr(program, owner), hir_expr(program, place)],
        ),
        E::Call { callee, args, .. } => {
            let label = match callee {
                hir::Callee::User(func) => format!("call #{func}"),
                hir::Callee::Op(op) => format!("call {op:?}"),
            };
            node(label, args.iter().map(|a| hir_expr(program, a)).collect())
        }
        E::Not(value) => node("not", vec![hir_expr(program, value)]),
        E::Ask { value, verb, .. } => node(
            format!("ask {}", program.names.name(*verb)),
            vec![hir_expr(program, value)],
        ),
        E::And(left, right) => node(
            "and",
            vec![hir_expr(program, left), hir_expr(program, right)],
        ),
        E::Or(left, right) => node(
            "or",
            vec![hir_expr(program, left), hir_expr(program, right)],
        ),
    }
}

pub fn types(program: &hir::Program) -> String {
    let found = crate::types::infer(program);
    let mut out = String::new();
    out.push_str("globals\n");
    for (slot, ty) in found.globals.iter().enumerate() {
        out.push_str(&format!("  {slot}: {ty:?}\n"));
    }
    for (id, function) in program.functions.iter().enumerate() {
        out.push_str(&format!(
            "fn #{id} {} -> {:?}\n",
            function.name, found.returns[id]
        ));
        for (slot, ty) in found.locals[id].iter().enumerate() {
            let role = if function.params.contains(&(slot as u32)) {
                "매개변수"
            } else {
                "지역"
            };
            out.push_str(&format!("  {role} {slot}: {ty:?}\n"));
        }
    }
    out
}
