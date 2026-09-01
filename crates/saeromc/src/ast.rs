use crate::diag::Span;
use crate::sig::Marker;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

#[derive(Clone, Debug)]
pub enum TemplatePart {
    Text(String),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub marker: Marker,
    pub expr: Expr,
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    pub verb: String,
    pub slots: Vec<Slot>,
    pub negated: bool,
    pub asks: bool,
    pub tail: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PassiveExpr {
    pub verb: String,
    pub head: Expr,
    pub slots: Vec<Slot>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Literal {
        value: Literal,
        span: Span,
    },
    Name {
        name: String,
        span: Span,
    },
    List {
        items: Vec<Expr>,
        span: Span,
    },
    Dict {
        entries: Vec<(String, Expr)>,
        span: Span,
    },
    Template {
        parts: Vec<TemplatePart>,
        span: Span,
    },
    Field {
        owner: Box<Expr>,
        name: String,
        span: Span,
    },
    Call(Box<CallExpr>),
    Passive(Box<PassiveExpr>),
    And {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Or {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. }
            | Expr::Name { span, .. }
            | Expr::List { span, .. }
            | Expr::Dict { span, .. }
            | Expr::Template { span, .. }
            | Expr::Field { span, .. }
            | Expr::And { span, .. }
            | Expr::Or { span, .. } => *span,
            Expr::Call(call) => call.span,
            Expr::Passive(passive) => passive.span,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            Expr::Name { name, .. } => Some(name),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub root: String,
    pub fields: Vec<String>,
    pub span: Span,
}

pub type Block = Vec<Stmt>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefKind {
    Verb,
    Predicate,
}

#[derive(Clone, Debug)]
pub enum LoopKind {
    Range {
        variable: String,
        start: Expr,
        stop: Expr,
        step: Option<Expr>,
    },
    While {
        test: Expr,
    },
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Declare {
        target: Target,
        value: Expr,
        span: Span,
    },
    Exec {
        calls: Vec<CallExpr>,
        span: Span,
    },
    Value {
        expr: Expr,
        span: Span,
    },
    If {
        branches: Vec<(Expr, Block)>,
        otherwise: Option<Block>,
        span: Span,
    },
    Loop {
        kind: LoopKind,
        body: Block,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Return {
        value: Expr,
        span: Span,
    },
    Define {
        name: String,
        kind: DefKind,
        params: Vec<(Marker, String)>,
        body: Block,
        span: Span,
    },
    Noun {
        name: String,
        owner: String,
        body: Block,
        span: Span,
    },
    With {
        call: CallExpr,
        name: String,
        body: Block,
        span: Span,
    },
    Import {
        module: String,
        names: Option<Vec<String>>,
        path: PathBuf,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Declare { span, .. }
            | Stmt::Exec { span, .. }
            | Stmt::Value { span, .. }
            | Stmt::If { span, .. }
            | Stmt::Loop { span, .. }
            | Stmt::Break { span }
            | Stmt::Continue { span }
            | Stmt::Return { span, .. }
            | Stmt::Define { span, .. }
            | Stmt::Noun { span, .. }
            | Stmt::With { span, .. }
            | Stmt::Import { span, .. } => *span,
        }
    }
}
