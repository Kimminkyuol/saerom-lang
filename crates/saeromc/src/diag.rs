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
}

impl Diag {
    pub fn new(kind: &'static str, msg: impl Into<String>, span: Span) -> Self {
        Diag {
            kind,
            msg: msg.into(),
            hint: None,
            span,
        }
    }

    pub fn lex(msg: impl Into<String>, span: Span) -> Self {
        Diag::new("어휘 오류", msg, span)
    }

    pub fn syntax(msg: impl Into<String>, span: Span) -> Self {
        Diag::new("구문 오류", msg, span)
    }

    pub fn name(msg: impl Into<String>, span: Span) -> Self {
        Diag::new("이름 오류", msg, span)
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render(&self, source: &str, path: &str) -> String {
        let mut out = format!(
            "{}: {}\n  {}:{}:{}\n",
            self.kind,
            self.msg,
            path,
            self.span.line,
            self.span.col + 1
        );
        if let Some(text) = source.lines().nth(self.span.line.saturating_sub(1)) {
            let pad: usize = text.chars().take(self.span.col).map(terminal_width).sum();
            let mark: usize = text
                .chars()
                .skip(self.span.col)
                .take(self.span.end.saturating_sub(self.span.col).max(1))
                .map(terminal_width)
                .sum();
            out.push_str(&format!("  {}\n", text));
            out.push_str(&format!(
                "  {}{}\n",
                " ".repeat(pad),
                "^".repeat(mark.max(1))
            ));
        }
        if let Some(hint) = &self.hint {
            out.push_str(&format!("  도움말: {}\n", hint));
        }
        out
    }
}

fn terminal_width(ch: char) -> usize {
    match ch as u32 {
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6 => 2,
        _ => 1,
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.msg)
    }
}

pub type Result<T> = std::result::Result<T, Diag>;
