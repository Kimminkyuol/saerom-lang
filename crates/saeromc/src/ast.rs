use crate::diag::Span;
use crate::lex::Num;

#[derive(Clone, Debug)]
pub enum Expr {
    Str(String),
    Number(Num),
    Name(String),
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub particle: Option<&'static str>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Call {
    pub verb: String,
    pub slots: Vec<Slot>,
    pub span: Span,
}

impl Call {
    pub fn signature(&self) -> Vec<&'static str> {
        let mut used: Vec<&'static str> =
            self.slots.iter().filter_map(|slot| slot.particle).collect();
        used.sort_unstable();
        used
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Exec(Vec<Call>),
}
