use crate::diag::{Diag, Result, Span};
use crate::words;

#[derive(Clone, Debug, PartialEq)]
pub enum Num {
    Int(i64),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Name(String),
    Verb {
        name: String,
        pos: &'static str,
        ending: &'static str,
    },
    Copula {
        ending: &'static str,
    },
    Particle {
        role: &'static str,
        canon: &'static str,
    },
    Keyword(String),
    Number(Num),
    Str(String),
    Symbol(char),
    Newline,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

impl Token {
    fn new(tok: Tok, line: usize, col: usize, end: usize) -> Self {
        Token {
            tok,
            span: Span::new(line, col, end),
        }
    }
}

#[derive(Default, Clone)]
pub struct Vocabulary {
    pub names: Vec<String>,
}

impl Vocabulary {
    fn knows(&self, word: &str) -> bool {
        self.names.iter().any(|n| n == word)
    }
}

#[derive(Clone, Copy)]
struct Splitting {
    take_particle: bool,
    take_copula: bool,
}

impl Splitting {
    const WHOLE: Splitting = Splitting {
        take_particle: true,
        take_copula: true,
    };
}

pub fn tokenize(source: &str, vocab: &Vocabulary) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    for (index, text) in source.lines().enumerate() {
        let line = index + 1;
        let produced = scan_line(text, line, vocab)?;
        let Some(last) = produced.last() else {
            continue;
        };
        let end = last.span.end;
        tokens.extend(produced);
        tokens.push(Token::new(Tok::Newline, line, end, end));
    }
    let line = source.lines().count() + 1;
    tokens.push(Token::new(Tok::Eof, line, 0, 0));
    Ok(tokens)
}

fn scan_line(text: &str, line: usize, vocab: &Vocabulary) -> Result<Vec<Token>> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '#' {
            break;
        }
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '"' {
            let (token, next) = scan_string(&chars, i, line)?;
            out.push(token);
            i = next;
        } else if ch == '-' && matches!(chars.get(i + 1), Some(c) if c.is_ascii_digit()) {
            let (token, next) = scan_number(&chars, i, line);
            out.push(token);
            i = next;
        } else if "[],:.{}".contains(ch) {
            out.push(Token::new(Tok::Symbol(ch), line, i, i + 1));
            i += 1;
        } else if is_word_char(ch) {
            let j = word_end(&chars, i);
            let chunk: String = chars[i..j].iter().collect();
            out.extend(split_word(&chunk, line, i, j, Splitting::WHOLE, vocab)?);
            i = j;
        } else {
            return Err(Diag::lex(
                format!("쓸 수 없는 글자: {ch:?}"),
                Span::new(line, i, i + 1),
            ));
        }
    }
    Ok(out)
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn word_end(chars: &[char], start: usize) -> usize {
    let numeric = chars[start].is_ascii_digit();
    let mut j = start;
    while j < chars.len() {
        let dotted_digit = chars[j] == '.'
            && numeric
            && matches!(chars.get(j + 1), Some(c) if c.is_ascii_digit());
        if !is_word_char(chars[j]) && !dotted_digit {
            break;
        }
        j += 1;
    }
    j
}

fn scan_number(chars: &[char], start: usize, line: usize) -> (Token, usize) {
    let j = word_end(chars, start + 1);
    let raw: String = chars[start..j].iter().collect();
    (
        Token::new(Tok::Number(parse_number(&raw)), line, start, j),
        j,
    )
}

fn parse_number(raw: &str) -> Num {
    match raw.parse::<i64>() {
        Ok(value) => Num::Int(value),
        Err(_) => Num::Float(raw.parse().unwrap_or(0.0)),
    }
}

fn scan_string(chars: &[char], start: usize, line: usize) -> Result<(Token, usize)> {
    let mut text = String::new();
    let mut j = start + 1;
    while j < chars.len() && chars[j] != '"' {
        if chars[j] == '\\' && j + 1 < chars.len() {
            text.push(unescape(chars[j + 1]));
            j += 2;
        } else if chars[j] == '{' {
            return Err(
                Diag::lex("보간은 아직 지원하지 않음", Span::new(line, j, j + 1))
                    .with_hint("중괄호를 글자로 쓰려면 '\\{'"),
            );
        } else {
            text.push(chars[j]);
            j += 1;
        }
    }
    if j >= chars.len() {
        return Err(Diag::lex(
            "따옴표가 닫히지 않음",
            Span::new(line, start, start + 1),
        ));
    }
    Ok((Token::new(Tok::Str(text), line, start, j + 1), j + 1))
}

fn unescape(ch: char) -> char {
    match ch {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        other => other,
    }
}

fn split_word(
    chunk: &str,
    line: usize,
    col: usize,
    end: usize,
    splitting: Splitting,
    vocab: &Vocabulary,
) -> Result<Vec<Token>> {
    let one = |tok| Ok(vec![Token::new(tok, line, col, end)]);

    if words::is_keyword(chunk) {
        return one(Tok::Keyword(chunk.into()));
    }
    if vocab.knows(chunk) || words::COMPARATIVES.iter().any(|&(w, _, _)| w == chunk) {
        return one(Tok::Name(chunk.into()));
    }
    for &(form, ending) in words::HADA_FORMS {
        if let Some(head) = body_before(chunk, form) {
            return one(Tok::Verb {
                name: format!("{head}하다"),
                pos: "verb",
                ending,
            });
        }
    }
    for &(form, ending) in words::DOEDA_FORMS {
        if let Some(head) = body_before(chunk, form) {
            return one(Tok::Verb {
                name: format!("{head}되다"),
                pos: "passive",
                ending,
            });
        }
    }
    if let Some(&(_, ending)) = words::COPULA.iter().find(|&&(f, _)| f == chunk) {
        return one(Tok::Copula { ending });
    }
    if splitting.take_copula {
        for &(form, ending) in words::COPULA {
            if let Some(head) = body_before(chunk, form) {
                let cut = end - form.chars().count();
                let rest = Splitting {
                    take_copula: false,
                    ..splitting
                };
                let mut out = split_word(head, line, col, cut, rest, vocab)?;
                out.push(Token::new(Tok::Copula { ending }, line, cut, end));
                return Ok(out);
            }
        }
    }
    if let Some((role, canon)) = words::particle(chunk) {
        return one(Tok::Particle { role, canon });
    }
    if is_number(chunk) {
        return one(Tok::Number(parse_number(chunk)));
    }
    if splitting.take_particle {
        for (form, role, canon) in words::particles_by_length() {
            if let Some(head) = body_before(chunk, &form) {
                let cut = end - form.chars().count();
                let rest = Splitting {
                    take_particle: false,
                    ..splitting
                };
                let mut out = split_word(head, line, col, cut, rest, vocab)?;
                out.push(Token::new(Tok::Particle { role, canon }, line, cut, end));
                return Ok(out);
            }
        }
    }
    one(Tok::Name(chunk.into()))
}

fn body_before<'a>(chunk: &'a str, suffix: &str) -> Option<&'a str> {
    chunk.strip_suffix(suffix).filter(|head| !head.is_empty())
}

fn is_number(text: &str) -> bool {
    let stripped = text.replacen('.', "", 1);
    !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_digit())
}
