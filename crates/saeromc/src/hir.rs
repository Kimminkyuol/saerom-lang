use crate::builtins::Builtin;
use crate::diag::Span;
use crate::intern::{Interner, Symbol};
use std::collections::HashMap;
use std::rc::Rc;

pub type FuncId = u32;
pub type GlobalId = u32;
pub type LocalId = u32;
pub type ModuleId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callee {
    User(FuncId),
    Op(Builtin),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Nothing,
    Local(LocalId),
    Global(GlobalId),
    Table(Vec<Expr>, Vec<(Symbol, Expr)>),
    Template(Vec<Expr>),
    Field {
        owner: Box<Expr>,
        field: Symbol,
        span: Span,
    },
    Index {
        owner: Box<Expr>,
        place: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Callee,
        args: Vec<Expr>,
        span: Span,
    },
    Not(Box<Expr>),
    Ask {
        value: Box<Expr>,
        verb: Symbol,
        span: Span,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Local(LocalId),
    Global(GlobalId),
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Set {
        place: Place,
        value: Expr,
    },
    SetField {
        owner: Expr,
        field: Symbol,
        value: Expr,
        span: Span,
    },
    Eval(Expr),
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        otherwise: Option<Vec<Stmt>>,
    },
    Range {
        place: Place,
        start: Expr,
        stop: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
        span: Span,
    },
    While {
        test: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Return {
        value: Expr,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Verb,
    Noun,
}

pub struct Function {
    pub name: Rc<str>,
    pub kind: Kind,
    pub module: ModuleId,
    pub params: Vec<LocalId>,
    pub locals: u32,
    pub body: Vec<Stmt>,
    pub span: Span,
}

pub struct Module {
    pub name: Rc<str>,
    pub path: Rc<str>,
    pub source: Rc<str>,
    pub init: Vec<Stmt>,
    pub nouns: HashMap<Symbol, FuncId>,
}

pub struct Program {
    pub modules: Vec<Module>,
    pub functions: Vec<Function>,
    pub names: Interner,
    pub globals: u32,
    pub order: Vec<ModuleId>,
    pub root: ModuleId,
}
