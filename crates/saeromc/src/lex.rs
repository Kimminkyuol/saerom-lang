use crate::diag::{Diag, Result, Span};
use crate::hangul::{to_nfc, Ending, Pos};
use crate::msg;
use crate::words::{self, FormTable};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum Num {
    Int(i64),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    Expr { source: String, span: Span },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Name(String),
    Verb {
        name: String,
        pos: Pos,
        ending: Ending,
    },
    Copula {
        ending: Ending,
    },
    Particle {
        role: &'static str,
        canon: &'static str,
    },
    Keyword(String),
    Number(Num),
    Str(String),
    Template(Vec<Part>),
    Symbol(char),
    Indent(usize),
    Dedent(usize),
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
    pub names: HashSet<String>,
    pub stems: HashSet<String>,
    forms: FormTable,
}

impl Vocabulary {
    // 이름이 어떤 동사의 활용형과 같은지. split_word 에서 이름이 이기므로,
    // 겹치면 그 동사가 파일 전체에서 조용히 사라진다.
    pub fn verb_named(&self, word: &str) -> Option<&str> {
        words::builtin_forms()
            .get(word)
            .or_else(|| self.forms.get(word))
            .map(|(name, _, _)| name.as_str())
    }

    pub fn new(names: HashSet<String>, stems: HashSet<String>) -> Self {
        let forms = words::stem_forms(&stems);
        Vocabulary {
            names,
            stems,
            forms,
        }
    }
}

struct Lexer<'a> {
    vocab: &'a Vocabulary,
}

#[derive(Clone, Copy)]
struct Splitting {
    take_particle: bool,
    take_copula: bool,
}

pub fn ready(source: &str) -> String {
    to_nfc(source).replace('\t', "    ")
}

pub fn tokenize(source: &str, vocab: &Vocabulary) -> Result<Vec<Token>> {
    let source = ready(source);
    let lexer = Lexer { vocab };

    let lines: Vec<&str> = source.split('\n').collect();
    let mut tokens = Vec::new();
    let mut indents = vec![0usize];
    let mut at_statement_start = true;
    let mut statement_indent = 0usize;

    for (index, text) in lines.iter().enumerate() {
        let line = index + 1;
        let chars: Vec<char> = text.chars().collect();
        let depth = chars.iter().take_while(|&&c| c == ' ').count();
        if chars[depth..].is_empty() || chars[depth] == '#' {
            continue;
        }
        if !at_statement_start && depth <= statement_indent {
            // 마침표를 빠뜨리면 다음 줄이 통째로 앞 문장에 붙어, 오류가 한참
            // 뒤에서 엉뚱한 것을 가리킨다. 이어지는 줄은 더 깊게 들여쓴다.
            return Err(Diag::lex(msg::MISSING_PERIOD, Span::new(line, 0, depth)));
        }
        if at_statement_start {
            statement_indent = depth;
            if depth > *indents.last().unwrap() {
                indents.push(depth);
                tokens.push(Token::new(Tok::Indent(depth), line, 0, depth));
            }
            while depth < *indents.last().unwrap() {
                indents.pop();
                tokens.push(Token::new(Tok::Dedent(depth), line, 0, depth));
            }
            if depth != *indents.last().unwrap() {
                return Err(Diag::lex(msg::INDENT_ODD, Span::new(line, 0, depth)));
            }
        }

        let produced = lexer.scan_line(&chars, line)?;
        at_statement_start = ends_statement(&produced);
        let end = produced.last().map_or(0, |token| token.span.end);
        tokens.extend(produced);
        if at_statement_start {
            tokens.push(Token::new(Tok::Newline, line, end, end));
        }
    }

    let line = lines.len();
    while indents.len() > 1 {
        indents.pop();
        tokens.push(Token::new(Tok::Dedent(0), line, 0, 0));
    }
    tokens.push(Token::new(Tok::Eof, line, 0, 0));
    Ok(tokens)
}

fn ends_statement(produced: &[Token]) -> bool {
    let Some(last) = produced.last() else {
        return false;
    };
    if matches!(last.tok, Tok::Symbol('.') | Tok::Symbol(':')) {
        return true;
    }
    produced.len() == 1
        && matches!(
            last.tok,
            Tok::Number(_) | Tok::Str(_) | Tok::Template(_) | Tok::Keyword(_) | Tok::Name(_)
        )
}

impl Lexer<'_> {
    fn knows(&self, word: &str) -> bool {
        self.vocab.names.contains(word) || words::FIELDS.contains(&word)
    }

    fn scan_line(&self, chars: &[char], line: usize) -> Result<Vec<Token>> {
        let mut out: Vec<Token> = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            if ch == '#' {
                break;
            }
            if ch == ' ' {
                i += 1;
            } else if ch == '"' {
                let (token, next) = scan_string(chars, i, line)?;
                out.push(token);
                i = next;
            } else if ch == '-' && matches!(chars.get(i + 1), Some(c) if c.is_ascii_digit()) {
                let j = digits_end(chars, i + 1);
                let raw: String = chars[i..j].iter().collect();
                out.push(Token::new(
                    Tok::Number(number(&raw, line, i, j)?),
                    line,
                    i,
                    j,
                ));
                i = j;
            } else if ".:()".contains(ch) {
                out.push(Token::new(Tok::Symbol(ch), line, i, i + 1));
                i += 1;
            } else if is_word_char(ch) {
                let j = word_end(chars, i);
                let chunk: String = chars[i..j].iter().collect();
                let splitting = Splitting {
                    take_particle: true,
                    take_copula: true,
                };
                out.extend(self.split_word(&chunk, line, i, j, splitting)?);
                i = j;
            } else {
                return Err(Diag::lex(
                    msg::bad_char(&format!("{ch:?}")),
                    Span::new(line, i, i + 1),
                ));
            }
        }
        Ok(out)
    }

    fn split_word(
        &self,
        chunk: &str,
        line: usize,
        col: usize,
        end: usize,
        splitting: Splitting,
    ) -> Result<Vec<Token>> {
        let one = |tok| Ok(vec![Token::new(tok, line, col, end)]);

        if words::is_keyword(chunk) {
            return one(Tok::Keyword(chunk.into()));
        }
        if self.knows(chunk) || words::COMPARATIVES.iter().any(|&(w, _, _)| w == chunk) {
            return one(Tok::Name(chunk.into()));
        }
        for table in [words::builtin_forms(), &self.vocab.forms] {
            if let Some((name, pos, ending)) = table.get(chunk) {
                return one(Tok::Verb {
                    name: name.clone(),
                    pos: *pos,
                    ending: *ending,
                });
            }
        }
        for &(form, ending) in words::HADA_FORMS {
            if let Some(head) = body_before(chunk, form) {
                return one(Tok::Verb {
                    name: format!("{head}하다"),
                    pos: Pos::Verb,
                    ending,
                });
            }
        }
        for &(form, ending) in words::DOEDA_FORMS {
            if let Some(head) = body_before(chunk, form) {
                return one(Tok::Verb {
                    name: format!("{head}되다"),
                    pos: Pos::Passive,
                    ending,
                });
            }
        }
        if let Some(&(_, ending)) = words::COPULA.iter().find(|&&(f, _)| f == chunk) {
            return one(Tok::Copula { ending });
        }
        if splitting.take_copula {
            if let Some((form, ending)) = words::copula_suffix(chunk, &|head| self.knows(head))
            {
                let cut = end - form.chars().count();
                let head = chunk.strip_suffix(form).unwrap();
                let again = words::COPULA
                    .iter()
                    .any(|&(form, _)| head.strip_suffix(form).is_some_and(words::is_keyword));
                let rest = Splitting {
                    take_copula: again,
                    ..splitting
                };
                let mut out = self.split_word(head, line, col, cut, rest)?;
                out.push(Token::new(Tok::Copula { ending }, line, cut, end));
                return Ok(out);
            }
        }
        if let Some((role, canon)) = words::particle(chunk) {
            return one(Tok::Particle { role, canon });
        }
        if is_number(chunk) {
            return one(Tok::Number(number(chunk, line, col, end)?));
        }
        if splitting.take_particle {
            // `까지의`는 이름으로 뭉쳐 버려서 오류가 엉뚱한 곳을 가리킨다.
            for tail in ["부터의", "까지의"] {
                if body_before(chunk, tail).is_some() {
                    return Err(Diag::lex(
                        msg::RANGE_GENITIVE,
                        Span::new(line, end - tail.chars().count(), end),
                    ));
                }
            }
            for &(form, role, canon) in words::particles_by_length() {
                if let Some(head) = body_before(chunk, form) {
                    let cut = end - form.chars().count();
                    let rest = Splitting {
                        take_particle: false,
                        ..splitting
                    };
                    let mut out = self.split_word(head, line, col, cut, rest)?;
                    out.push(Token::new(Tok::Particle { role, canon }, line, cut, end));
                    return Ok(out);
                }
            }
        }
        one(Tok::Name(chunk.into()))
    }
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

fn digits_end(chars: &[char], start: usize) -> usize {
    let mut j = start;
    while j < chars.len() {
        let dotted =
            chars[j] == '.' && matches!(chars.get(j + 1), Some(c) if c.is_ascii_digit());
        if !chars[j].is_ascii_digit() && !dotted {
            break;
        }
        j += 1;
    }
    j
}

fn number(raw: &str, line: usize, col: usize, end: usize) -> Result<Num> {
    if let Ok(value) = raw.parse::<i64>() {
        return Ok(Num::Int(value));
    }
    raw.parse::<f64>()
        .map(Num::Float)
        .map_err(|_| Diag::lex(msg::not_number(raw), Span::new(line, col, end)))
}

fn scan_string(chars: &[char], start: usize, line: usize) -> Result<(Token, usize)> {
    let mut text = String::new();
    let mut parts: Vec<Part> = Vec::new();
    let mut j = start + 1;
    while j < chars.len() && chars[j] != '"' {
        if chars[j] == '\\' && j + 1 < chars.len() {
            text.push(unescape(chars[j + 1]));
            j += 2;
            continue;
        }
        if chars[j] != '{' {
            text.push(chars[j]);
            j += 1;
            continue;
        }
        let close = matching_brace(chars, j)
            .ok_or_else(|| Diag::lex(msg::BRACE_OPEN, Span::new(line, j, j + 1)))?;
        let raw: String = chars[j + 1..close].iter().collect();
        let inner = raw.trim();
        if inner.is_empty() {
            return Err(Diag::lex(msg::BRACE_EMPTY, Span::new(line, j, close + 1)));
        }
        let offset = j + 1 + raw.chars().take_while(|c| c.is_whitespace()).count();
        parts.push(Part::Text(std::mem::take(&mut text)));
        parts.push(Part::Expr {
            source: inner.to_string(),
            span: Span::new(line, offset, offset + inner.chars().count()),
        });
        j = close + 1;
    }
    if j >= chars.len() {
        return Err(Diag::lex(
            msg::QUOTE_OPEN,
            Span::new(line, start, start + 1),
        ));
    }
    if parts.is_empty() {
        return Ok((Token::new(Tok::Str(text), line, start, j + 1), j + 1));
    }
    parts.push(Part::Text(text));
    parts.retain(|part| !matches!(part, Part::Text(text) if text.is_empty()));
    Ok((Token::new(Tok::Template(parts), line, start, j + 1), j + 1))
}

fn matching_brace(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 1;
    for (offset, &ch) in chars.iter().enumerate().skip(open + 1) {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn unescape(ch: char) -> char {
    match ch {
        'n' => '\n',
        't' => '\t',
        other => other,
    }
}

fn body_before<'a>(chunk: &'a str, suffix: &str) -> Option<&'a str> {
    chunk.strip_suffix(suffix).filter(|head| !head.is_empty())
}

fn is_number(text: &str) -> bool {
    let stripped = text.replacen('.', "", 1);
    !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_digit())
}
