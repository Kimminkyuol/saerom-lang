use crate::msg;
use crate::report::Report;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub end: usize,
}

impl Span {
    pub fn new(line: usize, col: usize, end: usize) -> Self {
        Span { line, col, end }
    }
}

#[derive(Clone, Debug)]
pub struct Diag {
    pub kind: &'static str,
    pub msg: String,
    pub hint: Option<String>,
    pub span: Span,
    pub unit: Option<usize>,
}

impl Diag {
    pub fn new(kind: &'static str, msg: impl Into<String>, span: Span) -> Self {
        Diag {
            kind,
            msg: msg.into(),
            hint: None,
            span,
            unit: None,
        }
    }

    pub fn lex(msg: impl Into<String>, span: Span) -> Self {
        Diag::new(msg::LEX, msg, span)
    }

    pub fn syntax(msg: impl Into<String>, span: Span) -> Self {
        Diag::new(msg::SYNTAX, msg, span)
    }

    pub fn name(msg: impl Into<String>, span: Span) -> Self {
        Diag::new(msg::NAME, msg, span)
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render(&self, source: &str, path: &str) -> String {
        Report {
            kind: self.kind,
            msg: &self.msg,
            hint: self.hint.as_deref(),
            path,
            source: Some(source),
            line: self.span.line,
            col: self.span.col,
            end: self.span.end,
        }
        .render()
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.msg)
    }
}

pub type Result<T> = std::result::Result<T, Diag>;

pub fn suggest<'a>(word: &str, known: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let limit = (word.chars().count() / 2).max(1);
    known
        .into_iter()
        .map(|found| (distance(word, found), found))
        .filter(|&(cost, _)| cost <= limit)
        .min_by_key(|&(cost, found)| (cost, found.len()))
        .map(|(_, found)| found)
}

fn distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut row: Vec<usize> = (0..=right.len()).collect();
    for (i, a) in left.chars().enumerate() {
        let mut corner = row[0];
        row[0] = i + 1;
        for (j, &b) in right.iter().enumerate() {
            let next = if a == b {
                corner
            } else {
                corner.min(row[j]).min(row[j + 1]) + 1
            };
            corner = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[right.len()]
}
