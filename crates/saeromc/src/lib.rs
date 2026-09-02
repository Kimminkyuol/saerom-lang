pub mod ast;
pub mod builtins;
pub mod diag;
pub mod dump;
pub mod emit;
pub mod hangul;
pub mod hir;
pub mod intern;
pub mod lex;
pub mod load;
pub mod parse;
pub mod prescan;
pub mod resolve;
pub mod sig;
pub mod types;
pub mod words;

pub use saerom_msg::{msg, report};

use diag::Diag;
use std::path::Path;

pub struct Failure {
    pub loaded: Option<load::Loaded>,
    pub errors: Vec<Diag>,
}

impl Failure {
    pub fn render(&self, source: &str, path: &str) -> String {
        let mut out = match &self.loaded {
            Some(loaded) => loaded.render(&self.errors),
            None => self
                .errors
                .iter()
                .map(|error| error.render(source, path))
                .collect::<Vec<_>>()
                .join("\n"),
        };
        if self.errors.len() > 1 {
            out.push_str(&report::plain(
                msg::ERROR,
                &msg::aborting(self.errors.len()),
            ));
        }
        out
    }
}

pub fn tokens(source: &str, base_dir: Option<&Path>) -> diag::Result<Vec<lex::Token>> {
    let program = prescan::survey(source, base_dir)?;
    lex::tokenize(source, &program.vocab)
}

pub fn front(source: &str, base_dir: Option<&Path>) -> Result<Vec<ast::Stmt>, Vec<Diag>> {
    let single = |error| vec![error];
    let program = prescan::survey(source, base_dir).map_err(single)?;
    let tokens = lex::tokenize(source, &program.vocab).map_err(single)?;
    let parsed = parse::parse(&tokens, &program, base_dir);
    if parsed.errors.is_empty() {
        Ok(parsed.statements)
    } else {
        Err(parsed.errors)
    }
}

pub fn analyze(
    source: &str,
    path: Option<&Path>,
) -> Result<(load::Loaded, hir::Program), Failure> {
    let loaded = load::load(source, path).map_err(|error| Failure {
        loaded: None,
        errors: vec![error],
    })?;
    if !loaded.errors.is_empty() {
        let errors = loaded.errors.clone();
        return Err(Failure {
            loaded: Some(loaded),
            errors,
        });
    }
    match resolve::resolve(&loaded) {
        Ok(program) => Ok((loaded, program)),
        Err(errors) => Err(Failure {
            loaded: Some(loaded),
            errors,
        }),
    }
}

pub fn compile(
    source: &str,
    path: Option<&Path>,
    triple: &str,
    frames: bool,
) -> Result<String, Failure> {
    let (loaded, program) = analyze(source, path)?;
    emit::emit(&program, triple, frames).map_err(|errors| Failure {
        loaded: Some(loaded),
        errors,
    })
}
