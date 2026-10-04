//! 서식. 들여쓰기·줄 이음·띄어쓰기만 고친다. 낱말이 그대로인지 다시 훑어 확인한다.

use crate::diag::{Diag, Result, Span};
use crate::lex::{ready, Tok};
use crate::msg;
use crate::prescan::survey;
use std::path::Path;

const STEP: usize = 4;

pub fn format(source: &str, base_dir: Option<&Path>) -> Result<String> {
    let text = ready(source);
    let before = shape(&text, base_dir)?;
    // 띄어쓰기를 고친 것이 낱말을 바꾸면 들여쓰기만 고친다.
    for tidy in [true, false] {
        let made = lay_out(&text, &before, tidy);
        if shape(&made, base_dir).is_ok_and(|after| after.tokens == before.tokens) {
            return Ok(made);
        }
    }
    Err(Diag::syntax(msg::FORMAT_FAILED, Span::default()))
}

struct Shape {
    tokens: Vec<Tok>,
    // 줄(1부터)마다: 낱말이 있는지, 문장을 끝내는지
    code: Vec<bool>,
    ends: Vec<bool>,
}

fn shape(text: &str, base_dir: Option<&Path>) -> Result<Shape> {
    let program = survey(text, base_dir)?;
    let count = text.split('\n').count() + 2;
    let mut found = Shape {
        tokens: Vec::new(),
        code: vec![false; count],
        ends: vec![false; count],
    };
    for token in program.tokens {
        let line = token.span.line.min(count - 1);
        match token.tok {
            Tok::Newline => found.ends[line] = true,
            Tok::Indent(_) => found.tokens.push(Tok::Indent(0)),
            Tok::Dedent(_) => found.tokens.push(Tok::Dedent(0)),
            Tok::Eof => {}
            tok => {
                found.code[line] = true;
                found.tokens.push(tok);
            }
        }
    }
    Ok(found)
}

enum Held {
    Blank,
    Comment(String),
}

fn lay_out(text: &str, shape: &Shape, tidy: bool) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut held: Vec<Held> = Vec::new();
    let mut stack = vec![0usize];
    let mut level = 0;
    let mut starting = true;
    let mut opened = false;
    for (index, raw) in text.split('\n').enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            held.push(Held::Blank);
            continue;
        }
        if !shape.code[line] {
            held.push(Held::Comment(trimmed.to_string()));
            continue;
        }
        let depth = raw.chars().take_while(|&c| c == ' ').count();
        let indent = if starting {
            while depth < *stack.last().unwrap() {
                stack.pop();
            }
            if depth > *stack.last().unwrap() {
                stack.push(depth);
            }
            level = stack.len() - 1;
            level * STEP
        } else {
            (level + 1) * STEP
        };
        release(&mut out, &mut held, indent, opened);
        let body = if tidy {
            spaced(trimmed)
        } else {
            trimmed.to_string()
        };
        opened = body
            .split('#')
            .next()
            .unwrap_or("")
            .trim_end()
            .ends_with(':');
        out.push(format!("{}{body}", " ".repeat(indent)));
        starting = shape.ends[line];
    }
    release(&mut out, &mut held, level * STEP, false);
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    let mut made = out.join("\n");
    made.push('\n');
    made
}

// 빈 줄은 하나로. 파일 처음과 블록 머리 바로 뒤에는 두지 않는다.
fn release(out: &mut Vec<String>, held: &mut Vec<Held>, indent: usize, opened: bool) {
    let mut fresh = opened;
    for item in held.drain(..) {
        match item {
            Held::Blank => {
                if !fresh && !out.is_empty() && out.last().is_some_and(|last| !last.is_empty())
                {
                    out.push(String::new());
                }
            }
            Held::Comment(text) => {
                fresh = false;
                out.push(format!("{}{text}", " ".repeat(indent)));
            }
        }
    }
}

// 글 밖의 빈칸을 하나로, 괄호 안쪽과 `.` `:` 앞의 빈칸은 없앤다. 주석 앞은 두 칸.
fn spaced(line: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    let mut brace = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            out.push(ch);
            match ch {
                '\\' => out.extend(chars.next()),
                '{' => brace += 1,
                '}' => brace = brace.saturating_sub(1),
                '"' if brace == 0 => quoted = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => {
                quoted = true;
                out.push(ch);
            }
            '#' => {
                let comment: String = std::iter::once(ch).chain(chars).collect();
                let code = out.trim_end().to_string();
                return if code.is_empty() {
                    comment.trim_end().to_string()
                } else {
                    format!("{code}  {}", comment.trim_end())
                };
            }
            ' ' if out.ends_with(' ') || out.ends_with('(') => {}
            '.' | ':' | ')' => {
                let kept = out.trim_end().len();
                out.truncate(kept);
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::format;

    #[test]
    fn lays_out() {
        let source = "\n\n수들은   3과 1이다 .\n# 주석\n수의 두배라는 것은:\n\n  # 안\n  답은 수에 \n          수를 더한 값이다.   # 끝\n  ( 답 )을 반환한다.\n\n\n\"a  b\"를 출력한다.\n\n";
        let want = "수들은 3과 1이다.\n# 주석\n수의 두배라는 것은:\n    # 안\n    답은 수에\n        수를 더한 값이다.  # 끝\n    (답)을 반환한다.\n\n\"a  b\"를 출력한다.\n";
        let made = format(source, None).unwrap();
        assert_eq!(made, want);
        assert_eq!(format(&made, None).unwrap(), made);
    }
}
