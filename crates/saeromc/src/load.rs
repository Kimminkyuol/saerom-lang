use crate::ast::Stmt;
use crate::diag::{Diag, Result, Span};
use crate::{lex, parse, prescan};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub type UnitId = usize;

#[derive(Clone)]
pub struct Unit {
    pub name: String,
    pub path: Option<PathBuf>,
    pub source: String,
    pub statements: Vec<Stmt>,
}

pub struct Loaded {
    pub units: Vec<Unit>,
    pub root: UnitId,
    pub errors: Vec<Diag>,
}

impl Loaded {
    pub fn unit_of(&self, path: &Path) -> Option<UnitId> {
        self.units
            .iter()
            .position(|unit| unit.path.as_deref() == Some(path))
    }

    pub fn render(&self, errors: &[Diag]) -> String {
        let mut out = String::new();
        for error in errors {
            let unit = error
                .unit
                .and_then(|id| self.units.get(id))
                .unwrap_or(&self.units[self.root]);
            let name = unit
                .path
                .as_deref()
                .map_or(unit.name.clone(), |path| path.display().to_string());
            out.push_str(&error.render(&unit.source, &name));
            out.push('\n');
        }
        out
    }
}

#[derive(Default)]
struct Walk {
    units: Vec<Unit>,
    done: HashMap<PathBuf, UnitId>,
    open: Vec<PathBuf>,
    errors: Vec<Diag>,
}

pub fn load(source: &str, path: Option<&Path>) -> Result<Loaded> {
    let mut walk = Walk::default();
    let name = path
        .and_then(|path| path.file_stem())
        .map_or("<입력>".to_string(), |stem| {
            stem.to_string_lossy().into_owned()
        });
    let whole = path.map(|path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    let root = walk.unit(
        name,
        whole.clone(),
        source.to_string(),
        whole.as_deref().and_then(Path::parent),
    )?;
    Ok(Loaded {
        units: walk.units,
        root,
        errors: walk.errors,
    })
}

impl Walk {
    fn unit(
        &mut self,
        name: String,
        path: Option<PathBuf>,
        source: String,
        base_dir: Option<&Path>,
    ) -> Result<UnitId> {
        let program = prescan::survey(&source, base_dir)?;
        let tokens = lex::tokenize(&source, &program.vocab)?;
        let parsed = parse::parse(&tokens, &program, base_dir);

        let id = self.units.len();
        self.units.push(Unit {
            name,
            path,
            source,
            statements: Vec::new(),
        });
        for mut error in parsed.errors {
            error.unit = Some(id);
            self.errors.push(error);
        }
        self.imports(&parsed.statements, id)?;
        self.units[id].statements = parsed.statements;
        Ok(id)
    }

    fn imports(&mut self, statements: &[Stmt], from: UnitId) -> Result<()> {
        for statement in statements {
            let Stmt::Import {
                module, path, span, ..
            } = statement
            else {
                continue;
            };
            let path = path.canonicalize().unwrap_or_else(|_| path.clone());
            if self.done.contains_key(&path) {
                continue;
            }
            if self.open.contains(&path) {
                let mut error = Diag::new("모듈 오류", format!("'{module}' 순환 참조"), *span);
                error.unit = Some(from);
                return Err(error);
            }
            let source = std::fs::read_to_string(&path).map_err(|error| {
                let mut diag = Diag::new(
                    "모듈 오류",
                    format!("'{module}'을 읽을 수 없음: {error}"),
                    *span,
                );
                diag.unit = Some(from);
                diag
            })?;
            self.open.push(path.clone());
            let made = self.unit(module.clone(), Some(path.clone()), source, path.parent());
            self.open.pop();
            self.done.insert(path, made?);
        }
        Ok(())
    }
}

pub fn span_of(statements: &[Stmt]) -> Span {
    statements.first().map(Stmt::span).unwrap_or_default()
}
