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
