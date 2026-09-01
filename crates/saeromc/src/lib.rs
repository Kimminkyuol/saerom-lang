pub mod ast;
pub mod diag;
pub mod dump;
pub mod emit;
pub mod hangul;
pub mod lex;
pub mod parse;
pub mod prescan;
pub mod words;

use diag::Result;
use std::path::Path;

pub fn tokens(source: &str, base_dir: Option<&Path>) -> Result<Vec<lex::Token>> {
    let vocabulary = prescan::prescan(source, base_dir)?;
    lex::tokenize(source, &vocabulary)
}

pub fn compile(source: &str, base_dir: Option<&Path>, triple: &str) -> Result<String> {
    let statements = parse::parse(&tokens(source, base_dir)?)?;
    emit::emit(&statements, triple)
}
