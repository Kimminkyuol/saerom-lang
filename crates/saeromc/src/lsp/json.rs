//! 쓰는 만큼만의 JSON.

use std::fmt::{self, Write};

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

static NULL: Json = Json::Null;

impl Json {
    pub fn get(&self, key: &str) -> &Json {
        match self {
            Json::Obj(pairs) => pairs
                .iter()
                .find(|(name, _)| name == key)
                .map_or(&NULL, |(_, value)| value),
            _ => &NULL,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn num(&self) -> Option<usize> {
        match self {
            Json::Num(value) if *value >= 0.0 => Some(*value as usize),
            _ => None,
        }
    }

    pub fn items(&self) -> &[Json] {
        match self {
            Json::Arr(items) => items,
            _ => &[],
        }
    }
}

pub fn obj<const N: usize>(pairs: [(&str, Json); N]) -> Json {
    Json::Obj(
        pairs
            .into_iter()
            .filter(|(_, value)| *value != Json::Null)
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

impl From<&str> for Json {
    fn from(text: &str) -> Self {
        Json::Str(text.to_string())
    }
}

impl From<String> for Json {
    fn from(text: String) -> Self {
        Json::Str(text)
    }
}

impl From<usize> for Json {
    fn from(value: usize) -> Self {
        Json::Num(value as f64)
    }
}

impl From<bool> for Json {
    fn from(value: bool) -> Self {
        Json::Bool(value)
    }
}

impl From<Vec<Json>> for Json {
    fn from(items: Vec<Json>) -> Self {
        Json::Arr(items)
    }
}

impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(value: Option<T>) -> Self {
        value.map_or(Json::Null, Into::into)
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Json::Null => f.write_str("null"),
            Json::Bool(value) => write!(f, "{value}"),
            Json::Num(value) if value.fract() == 0.0 && value.abs() < 1e15 => {
                write!(f, "{}", *value as i64)
            }
            Json::Num(value) => write!(f, "{value}"),
            Json::Str(text) => quote(text, f),
            Json::Arr(items) => {
                f.write_char('[')?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_char(',')?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_char(']')
            }
            Json::Obj(pairs) => {
                f.write_char('{')?;
                for (index, (name, value)) in pairs.iter().enumerate() {
                    if index > 0 {
                        f.write_char(',')?;
                    }
                    quote(name, f)?;
                    write!(f, ":{value}")?;
                }
                f.write_char('}')
            }
        }
    }
}

fn quote(text: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_char('"')?;
    for ch in text.chars() {
        match ch {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

pub fn parse(text: &str) -> Option<Json> {
    let mut reader = Reader {
        chars: text.chars().collect(),
        at: 0,
    };
    let value = reader.value()?;
    reader.blank();
    (reader.at == reader.chars.len()).then_some(value)
}

struct Reader {
    chars: Vec<char>,
    at: usize,
}

impl Reader {
    fn blank(&mut self) {
        while self.chars.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }

    fn next(&mut self) -> Option<char> {
        let ch = *self.chars.get(self.at)?;
        self.at += 1;
        Some(ch)
    }

    fn eat(&mut self, word: &str) -> bool {
        let found = word
            .chars()
            .enumerate()
            .all(|(index, ch)| self.chars.get(self.at + index) == Some(&ch));
        if found {
            self.at += word.chars().count();
        }
        found
    }

    fn value(&mut self) -> Option<Json> {
        self.blank();
        let first = *self.chars.get(self.at)?;
        match first {
            '{' => self.object(),
            '[' => self.array(),
            '"' => self.string().map(Json::Str),
            _ if self.eat("null") => Some(Json::Null),
            _ if self.eat("true") => Some(Json::Bool(true)),
            _ if self.eat("false") => Some(Json::Bool(false)),
            _ => self.number(),
        }
    }

    fn object(&mut self) -> Option<Json> {
        self.at += 1;
        let mut pairs = Vec::new();
        self.blank();
        if self.eat("}") {
            return Some(Json::Obj(pairs));
        }
        loop {
            self.blank();
            let name = self.string()?;
            self.blank();
            if !self.eat(":") {
                return None;
            }
            pairs.push((name, self.value()?));
            self.blank();
            match self.next()? {
                ',' => {}
                '}' => return Some(Json::Obj(pairs)),
                _ => return None,
            }
        }
    }

    fn array(&mut self) -> Option<Json> {
        self.at += 1;
        let mut items = Vec::new();
        self.blank();
        if self.eat("]") {
            return Some(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.blank();
            match self.next()? {
                ',' => {}
                ']' => return Some(Json::Arr(items)),
                _ => return None,
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        if self.next()? != '"' {
            return None;
        }
        let mut out = String::new();
        loop {
            match self.next()? {
                '"' => return Some(out),
                '\\' => match self.next()? {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'u' => {
                        let high = self.hex()?;
                        let code = if (0xD800..0xDC00).contains(&high) && self.eat("\\u") {
                            0x10000 + ((high - 0xD800) << 10) + (self.hex()? - 0xDC00)
                        } else {
                            high
                        };
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    other => out.push(other),
                },
                ch => out.push(ch),
            }
        }
    }

    fn hex(&mut self) -> Option<u32> {
        let digits: String = (0..4).map(|_| self.next()).collect::<Option<_>>()?;
        u32::from_str_radix(&digits, 16).ok()
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.at;
        while self
            .chars
            .get(self.at)
            .is_some_and(|c| c.is_ascii_digit() || "+-.eE".contains(*c))
        {
            self.at += 1;
        }
        let raw: String = self.chars[start..self.at].iter().collect();
        raw.parse().ok().map(Json::Num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let text = r#"{"a":[1,2.5,"글\n\"",true,null],"b":{"c":-3}}"#;
        let value = parse(text).unwrap();
        assert_eq!(value.get("b").get("c"), &Json::Num(-3.0));
        assert_eq!(value.to_string(), text);
        assert_eq!(parse(r#""👍""#), Some(Json::Str("👍".into())));
    }
}
