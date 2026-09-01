pub mod ast;
pub mod diag;
pub mod emit;
pub mod lex;
pub mod parse;
pub mod words;

use diag::Result;

pub fn compile(source: &str, triple: &str) -> Result<String> {
    let tokens = lex::tokenize(source, &lex::Vocabulary::default())?;
    let statements = parse::parse(&tokens)?;
    emit::emit(&statements, triple)
}
