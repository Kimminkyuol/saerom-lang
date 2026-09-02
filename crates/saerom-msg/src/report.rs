use crate::msg;

pub struct Report<'a> {
    pub kind: &'a str,
    pub msg: &'a str,
    pub hint: Option<&'a str>,
    pub path: &'a str,
    pub source: Option<&'a str>,
    pub line: usize,
    pub col: usize,
    pub end: usize,
}

impl Report<'_> {
    pub fn render(&self) -> String {
        let gutter = self.line.to_string().len();
        let bar = " ".repeat(gutter);
        let rule = blue(&format!("{bar} |"));
        let mut out = format!("{}: {}\n", red(self.kind), bold(self.msg));
        out.push_str(&format!(
            "{} {}:{}:{}\n",
            blue(&format!("{bar}-->")),
            self.path,
            self.line,
            self.col + 1
        ));
        let text = self
            .source
            .and_then(|whole| whole.lines().nth(self.line.saturating_sub(1)));
        if let Some(text) = text {
            let pad: usize = text.chars().take(self.col).map(width).sum();
            let mark: usize = text
                .chars()
                .skip(self.col)
                .take(self.end.saturating_sub(self.col).max(1))
                .map(width)
                .sum();
            let here = blue(&format!("{} |", self.line));
            out.push_str(&format!("{rule}\n{here} {text}\n"));
            out.push_str(&format!(
                "{rule} {}{}\n",
                " ".repeat(pad),
                red(&"^".repeat(mark.max(1)))
            ));
        }
        if let Some(hint) = self.hint {
            out.push_str(&format!("{rule}\n"));
            out.push_str(&format!(
                "{} {}: {hint}\n",
                blue(&format!("{bar} =")),
                bold(msg::HELP)
            ));
        }
        out
    }
}

pub fn plain(kind: &str, message: &str) -> String {
    format!("{}: {}\n", red(kind), bold(message))
}

pub fn note(message: &str) -> String {
    format!("{}: {message}\n", bold(msg::NOTE))
}

pub fn red(text: &str) -> String {
    tint("\x1b[1;31m", text)
}

pub fn blue(text: &str) -> String {
    tint("\x1b[1;34m", text)
}

pub fn bold(text: &str) -> String {
    tint("\x1b[1m", text)
}

fn tint(code: &str, text: &str) -> String {
    if colored() {
        format!("{code}{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn colored() -> bool {
    use std::io::IsTerminal;
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
    })
}

fn width(ch: char) -> usize {
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
