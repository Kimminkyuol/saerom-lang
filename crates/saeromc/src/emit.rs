use crate::ast::{Call, Expr, Stmt};
use crate::diag::{Diag, Result};

struct Builtin {
    verb: &'static str,
    particles: &'static [&'static str],
    symbol: &'static str,
}

const BUILTINS: &[Builtin] = &[Builtin {
    verb: "출력하다",
    particles: &["를"],
    symbol: "sr_print_bytes",
}];

fn builtin(call: &Call) -> Option<&'static Builtin> {
    let used = call.signature();
    BUILTINS
        .iter()
        .find(|found| found.verb == call.verb && found.particles == used.as_slice())
}

#[derive(Default)]
pub struct Module {
    constants: Vec<String>,
    body: Vec<String>,
}

pub fn emit(statements: &[Stmt], triple: &str) -> Result<String> {
    let mut module = Module::default();
    for statement in statements {
        let Stmt::Exec(calls) = statement;
        for call in calls {
            module.call(call)?;
        }
    }
    let mut out = String::new();
    if !triple.is_empty() {
        out.push_str(&format!("target triple = \"{triple}\"\n\n"));
    }
    out.push_str(&module.constants.join(""));
    out.push_str("\ndeclare void @sr_print_bytes(ptr, i64)\n\ndefine i32 @main() {\nentry:\n");
    out.push_str(&module.body.join(""));
    out.push_str("  ret i32 0\n}\n");
    Ok(out)
}

impl Module {
    fn constant(&mut self, text: &str) -> (String, usize) {
        let name = format!("@.str{}", self.constants.len());
        let bytes = text.as_bytes();
        self.constants.push(format!(
            "{name} = private unnamed_addr constant [{} x i8] c\"{}\"\n",
            bytes.len(),
            escape(bytes)
        ));
        (name, bytes.len())
    }

    fn call(&mut self, call: &Call) -> Result<()> {
        let Some(found) = builtin(call) else {
            let used: Vec<&str> = call.signature();
            let shown = if used.is_empty() {
                "없음".into()
            } else {
                used.join(", ")
            };
            return Err(Diag::name(
                format!("동사 '{}'를 조사 {shown}로 부를 수 없음", call.verb),
                call.span,
            ));
        };
        let Expr::Str(text) = &call.slots[0].expr else {
            return Err(Diag::name(
                format!("'{}'의 인자는 아직 문자열만 됨", call.verb),
                call.slots[0].span,
            ));
        };
        let (name, len) = self.constant(text);
        self.body.push(format!(
            "  call void @{}(ptr {name}, i64 {len})\n",
            found.symbol
        ));
        Ok(())
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
