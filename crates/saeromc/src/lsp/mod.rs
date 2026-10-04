//! 언어 서버. `saeromc --lsp` 로 표준 입출력에서 돈다.

mod complete;
mod index;
mod json;
mod look;

use crate::diag::Span;
use crate::lex::Tok;
use index::Index;
use json::{obj, Json};
use look::Thing;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

struct Doc {
    path: Option<PathBuf>,
    typing: look::Typing,
    lines: Vec<String>,
    index: Index,
}

impl Doc {
    fn new(uri: &str, text: &str) -> Doc {
        let path = path_of(uri);
        Doc {
            index: index::build(text, path.as_deref()),
            typing: look::Typing::default(),
            lines: text
                .split('\n')
                .map(|line| line.trim_end_matches('\r').to_string())
                .collect(),
            path,
        }
    }

    fn line(&self, line: usize) -> &str {
        line.checked_sub(1)
            .and_then(|at| self.lines.get(at))
            .map_or("", String::as_str)
    }

    // 컴파일러 자리(탭은 네 칸)에서 UTF-16 자리로.
    fn utf16(&self, line: usize, col: usize) -> usize {
        let mut seen = 0;
        let mut out = 0;
        for ch in self.line(line).chars() {
            if seen >= col {
                break;
            }
            seen += match ch {
                '\t' => 4,
                '\u{feff}' => 0,
                _ => 1,
            };
            out += ch.len_utf16();
        }
        out + col.saturating_sub(seen)
    }

    fn col(&self, line: usize, utf16: usize) -> usize {
        let mut seen = 0;
        let mut out = 0;
        for ch in self.line(line).chars() {
            if seen >= utf16 {
                break;
            }
            seen += ch.len_utf16();
            out += match ch {
                '\t' => 4,
                '\u{feff}' => 0,
                _ => 1,
            };
        }
        out
    }

    fn prefix(&self, line: usize, utf16: usize) -> String {
        let mut seen = 0;
        self.line(line)
            .chars()
            .take_while(|ch| {
                seen += ch.len_utf16();
                seen <= utf16
            })
            .collect()
    }

    fn range(&self, span: Span) -> Json {
        let line = span.line.saturating_sub(1);
        obj([
            ("start", position(line, self.utf16(span.line, span.col))),
            ("end", position(line, self.utf16(span.line, span.end))),
        ])
    }
}

fn position(line: usize, character: usize) -> Json {
    obj([("line", line.into()), ("character", character.into())])
}

// 다른 글의 자리는 탭·NFD 를 따지지 않는다.
fn plain_range(span: Span) -> Json {
    let line = span.line.saturating_sub(1);
    obj([
        ("start", position(line, span.col)),
        ("end", position(line, span.end)),
    ])
}

#[derive(Default)]
struct Server {
    docs: HashMap<String, Doc>,
    // 다른 글에 내보낸 진단. 다음 번에 비운다.
    foreign: HashMap<String, HashSet<String>>,
    out: Vec<Json>,
}

pub fn serve() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout().lock();
    let mut server = Server::default();
    while let Some(text) = receive(&mut input)? {
        let Some(message) = json::parse(&text) else {
            continue;
        };
        let method = message.get("method").str().unwrap_or("");
        if method == "exit" {
            break;
        }
        let id = message.get("id").clone();
        let params = message.get("params");
        let answer = catch_unwind(AssertUnwindSafe(|| server.handle(method, params)));
        let reply = match answer {
            Ok(Some(result)) => Some(Json::Obj(vec![("result".into(), result)])),
            Ok(None) => (id != Json::Null && !method.is_empty()).then(|| {
                obj([(
                    "error",
                    obj([("code", Json::Num(-32601.0)), ("message", method.into())]),
                )])
            }),
            Err(_) => Some(Json::Obj(vec![("result".into(), Json::Null)])),
        };
        for note in std::mem::take(&mut server.out) {
            send(&mut output, note)?;
        }
        if let (Some(Json::Obj(mut pairs)), false) = (reply, id == Json::Null) {
            pairs.insert(0, ("id".into(), id));
            send(&mut output, Json::Obj(pairs))?;
        }
    }
    Ok(())
}

fn receive(input: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut length = None;
    loop {
        let mut header = String::new();
        if input.read_line(&mut header)? == 0 {
            return Ok(None);
        }
        let header = header.trim();
        if header.is_empty() {
            break;
        }
        if let Some(value) = header.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length.unwrap_or(0)];
    input.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

fn send(output: &mut impl Write, message: Json) -> io::Result<()> {
    let Json::Obj(mut pairs) = message else {
        return Ok(());
    };
    pairs.insert(0, ("jsonrpc".into(), "2.0".into()));
    let body = Json::Obj(pairs).to_string();
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

impl Server {
    fn handle(&mut self, method: &str, params: &Json) -> Option<Json> {
        let uri = params.get("textDocument").get("uri").str().unwrap_or("");
        match method {
            "initialize" => Some(capabilities()),
            "shutdown" => Some(Json::Null),
            "textDocument/didOpen" => {
                let text = params.get("textDocument").get("text").str().unwrap_or("");
                self.open(uri, text);
                None
            }
            "textDocument/didChange" => {
                let changes = params.get("contentChanges").items();
                if let Some(text) = changes.last().and_then(|change| change.get("text").str()) {
                    self.open(uri, text);
                }
                None
            }
            "textDocument/didClose" => {
                self.docs.remove(uri);
                self.publish(uri, Vec::new());
                None
            }
            "textDocument/completion" => self.completion(uri, params),
            "textDocument/hover" => self.hover(uri, params),
            "textDocument/definition" => self.definition(uri, params),
            "textDocument/references" => self.references(uri, params),
            "textDocument/documentHighlight" => self.highlights(uri, params),
            "textDocument/rename" => self.rename(uri, params),
            "textDocument/documentSymbol" => self.symbols(uri),
            "textDocument/formatting" => self.format(uri),
            "textDocument/semanticTokens/full" => self.semantic(uri),
            _ => None,
        }
    }

    fn open(&mut self, uri: &str, text: &str) {
        let mut doc = Doc::new(uri, text);
        if let Some(old) = self.docs.remove(uri) {
            doc.typing = old.typing;
        }
        let path = doc.path.clone();
        self.docs.insert(uri.to_string(), doc);
        self.diagnose(uri, text, path.as_deref());
    }

    fn diagnose(&mut self, uri: &str, text: &str, path: Option<&Path>) {
        let mut found: HashMap<String, Vec<Json>> = HashMap::new();
        let failure = match crate::analyze_typed(text, path) {
            Ok((_, program, types)) => {
                if let Some(doc) = self.docs.get_mut(uri) {
                    doc.typing = look::Typing::new(&program, &types);
                }
                None
            }
            Err(failure) => Some(failure),
        };
        if let Some(failure) = failure {
            for error in &failure.errors {
                let foreign = failure.loaded.as_ref().and_then(|loaded| {
                    let unit = error.unit.filter(|&unit| unit != loaded.root)?;
                    loaded.units[unit].path.as_deref().map(uri_of)
                });
                let range = match (&foreign, self.docs.get(uri)) {
                    (None, Some(doc)) => doc.range(error.span),
                    _ => plain_range(error.span),
                };
                let mut message = error.msg.clone();
                if let Some(hint) = &error.hint {
                    message.push_str(&format!("\n{hint}"));
                }
                found
                    .entry(foreign.unwrap_or_else(|| uri.to_string()))
                    .or_default()
                    .push(obj([
                        ("range", range),
                        ("severity", 1usize.into()),
                        ("source", "새롬".into()),
                        ("message", message.into()),
                    ]));
            }
        }
        let mine = found.remove(uri).unwrap_or_default();
        self.publish(uri, mine);
        let before = self.foreign.remove(uri).unwrap_or_default();
        for stale in before.difference(&found.keys().cloned().collect()) {
            self.publish(stale, Vec::new());
        }
        self.foreign
            .insert(uri.to_string(), found.keys().cloned().collect());
        for (other, list) in found {
            self.publish(&other, list);
        }
    }

    fn publish(&mut self, uri: &str, list: Vec<Json>) {
        self.out.push(obj([
            ("method", "textDocument/publishDiagnostics".into()),
            (
                "params",
                obj([("uri", uri.into()), ("diagnostics", list.into())]),
            ),
        ]));
    }

    // 커서의 줄(1부터)과 컴파일러 자리.
    fn spot<'a>(&'a self, uri: &str, params: &Json) -> Option<(&'a Doc, usize, usize)> {
        let doc = self.docs.get(uri)?;
        let at = params.get("position");
        let line = at.get("line").num()? + 1;
        let col = doc.col(line, at.get("character").num()?);
        Some((doc, line, col))
    }

    fn token_at<'a>(&'a self, uri: &str, params: &Json) -> Option<(&'a Doc, usize)> {
        let (doc, line, col) = self.spot(uri, params)?;
        let (at, _) = doc.index.at(line, col)?;
        Some((doc, at))
    }

    fn completion(&self, uri: &str, params: &Json) -> Option<Json> {
        let doc = self.docs.get(uri)?;
        let at = params.get("position");
        let line = at.get("line").num()? + 1;
        let prefix = doc.prefix(line, at.get("character").num()?);
        let typed = params.get("context").get("triggerKind").num() == Some(2);
        Some(complete::complete(
            &doc.index,
            &prefix,
            line,
            doc.path.as_deref(),
            typed,
        ))
    }

    fn hover(&self, uri: &str, params: &Json) -> Option<Json> {
        let Some((doc, at)) = self.token_at(uri, params) else {
            return Some(Json::Null);
        };
        let Some(thing) = look::identify(&doc.index, at) else {
            return Some(Json::Null);
        };
        Some(obj([
            (
                "contents",
                obj([
                    ("kind", "markdown".into()),
                    ("value", look::hover(&thing, &doc.typing).into()),
                ]),
            ),
            ("range", doc.range(doc.index.tokens[at].span)),
        ]))
    }

    fn definition(&self, uri: &str, params: &Json) -> Option<Json> {
        let Some((doc, at)) = self.token_at(uri, params) else {
            return Some(Json::Null);
        };
        let place = |path: Option<&Path>, range| {
            let uri = path.map_or_else(|| uri.to_string(), uri_of);
            Some(obj([("uri", uri.into()), ("range", range)]))
        };
        match look::identify(&doc.index, at) {
            Some(Thing::Def(def)) => match &def.path {
                None => {
                    let span = look::pin(&doc.index, def.span.line, &def.name)
                        .map_or(def.span, |token| token.span);
                    place(None, doc.range(span))
                }
                Some(path) => place(Some(path), plain_range(look::foreign_pin(path, def))),
            },
            Some(Thing::Var(var)) => {
                let span = look::pin(&doc.index, var.span.line, &var.name)
                    .map_or(var.span, |token| token.span);
                place(None, doc.range(span))
            }
            Some(Thing::Module(_, path)) => place(Some(&path), plain_range(Span::new(1, 0, 0))),
            _ => Some(Json::Null),
        }
    }

    fn references(&self, uri: &str, params: &Json) -> Option<Json> {
        let found = self.everywhere(uri, params).unwrap_or_default();
        Some(Json::Arr(
            found
                .into_iter()
                .map(|(uri, range, _)| obj([("uri", uri.into()), ("range", range)]))
                .collect(),
        ))
    }

    // 같은 것을 가리키는 낱말들: (uri, 자리, 낱말). 정의는 그 글과 이 글의
    // 폴더에 있는 모든 글에서 찾는다.
    fn everywhere(&self, uri: &str, params: &Json) -> Option<Vec<(String, Json, Tok)>> {
        let (doc, at) = self.token_at(uri, params)?;
        let local = |doc: &Doc, uri: &str| {
            look::occurrences(&doc.index, at)
                .into_iter()
                .map(|token| (uri.to_string(), doc.range(token.span), token.tok.clone()))
                .collect()
        };
        let Some(Thing::Def(def)) = look::identify(&doc.index, at) else {
            return Some(local(doc, uri));
        };
        let (Some(here), name) = (doc.path.as_deref().map(real), def.name.clone()) else {
            return Some(local(doc, uri));
        };
        let home = def.path.as_deref().map_or_else(|| here.clone(), real);
        let mut files: Vec<PathBuf> = vec![here.clone()];
        for folder in [here.parent(), home.parent()].into_iter().flatten() {
            let Ok(entries) = std::fs::read_dir(folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = real(&entry.path());
                if path.extension().is_some_and(|ext| ext == "sr") && !files.contains(&path) {
                    files.push(path);
                }
            }
        }
        let mut found = Vec::new();
        for path in files {
            let open = self
                .docs
                .iter()
                .find(|(_, doc)| doc.path.as_deref().map(real).as_ref() == Some(&path));
            let made;
            let (other_uri, other) = match open {
                Some((uri, doc)) => (uri.clone(), doc),
                None => {
                    let Ok(text) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let uri = uri_of(&path);
                    made = Doc::new(&uri, &text);
                    (uri, &made)
                }
            };
            for (index, token) in other.index.tokens.iter().enumerate() {
                let Some(Thing::Def(seen)) = look::identify(&other.index, index) else {
                    continue;
                };
                let origin = seen.path.as_deref().map_or_else(|| path.clone(), real);
                if seen.name == name && origin == home {
                    found.push((
                        other_uri.clone(),
                        other.range(token.span),
                        token.tok.clone(),
                    ));
                }
            }
        }
        Some(found)
    }

    fn highlights(&self, uri: &str, params: &Json) -> Option<Json> {
        let Some((doc, at)) = self.token_at(uri, params) else {
            return Some(Json::Arr(Vec::new()));
        };
        Some(Json::Arr(
            look::occurrences(&doc.index, at)
                .into_iter()
                .map(|token| obj([("range", doc.range(token.span))]))
                .collect(),
        ))
    }

    // 동사는 쓰인 활용형 그대로 새 이름으로 다시 활용한다.
    fn rename(&self, uri: &str, params: &Json) -> Option<Json> {
        let (doc, at) = self.token_at(uri, params)?;
        let wanted = params.get("newName").str()?.trim();
        match look::identify(&doc.index, at)? {
            // 표준 모듈은 고치지 않는다.
            Thing::Def(def) if def.path.as_deref().is_none_or(|path| !standard(path)) => {}
            Thing::Var(_) => {}
            _ => return Some(Json::Null),
        }
        let forms = crate::words::forms_of(wanted);
        let mut changes: Vec<(String, Json)> = Vec::new();
        for (uri, range, tok) in self.everywhere(uri, params)? {
            let text = match tok {
                Tok::Verb { ending, .. } => forms
                    .iter()
                    .find(|(found, _)| *found == ending)
                    .map(|(_, form)| form.clone())?,
                _ => wanted.to_string(),
            };
            let edit = obj([("range", range), ("newText", text.into())]);
            match changes.iter_mut().find(|(known, _)| *known == uri) {
                Some((_, Json::Arr(edits))) => edits.push(edit),
                _ => changes.push((uri, Json::Arr(vec![edit]))),
            }
        }
        Some(obj([("changes", Json::Obj(changes))]))
    }

    fn symbols(&self, uri: &str) -> Option<Json> {
        let doc = self.docs.get(uri)?;
        let last = doc.lines.len();
        let mut found = Vec::new();
        for def in doc.index.defs.iter().filter(|def| def.path.is_none()) {
            let name = look::pin(&doc.index, def.span.line, &def.name)
                .map_or(def.span, |token| token.span);
            let end = def.lines.1.min(last).max(def.lines.0);
            let whole = obj([
                ("start", position(def.lines.0 - 1, 0)),
                (
                    "end",
                    position(end - 1, doc.line(end).encode_utf16().count()),
                ),
            ]);
            found.push(obj([
                ("name", def.name.as_str().into()),
                ("detail", def.shape().into()),
                ("kind", (if def.noun() { 7usize } else { 12 }).into()),
                ("range", whole),
                ("selectionRange", doc.range(name)),
            ]));
        }
        for var in doc.index.vars.iter().filter(|var| var.lines.is_none()) {
            let span = look::pin(&doc.index, var.span.line, &var.name)
                .map_or(var.span, |token| token.span);
            found.push(obj([
                ("name", var.name.as_str().into()),
                ("kind", 13usize.into()),
                ("range", doc.range(span)),
                ("selectionRange", doc.range(span)),
            ]));
        }
        Some(Json::Arr(found))
    }

    fn format(&self, uri: &str) -> Option<Json> {
        let doc = self.docs.get(uri)?;
        let text = doc.lines.join("\n");
        let base = doc.path.as_deref().and_then(Path::parent);
        let Ok(made) = crate::format(&text, base) else {
            return Some(Json::Null);
        };
        if made == text {
            return Some(Json::Arr(Vec::new()));
        }
        let whole = obj([
            ("start", position(0, 0)),
            ("end", position(doc.lines.len(), 0)),
        ]);
        Some(Json::Arr(vec![obj([
            ("range", whole),
            ("newText", made.into()),
        ])]))
    }

    fn semantic(&self, uri: &str) -> Option<Json> {
        let doc = self.docs.get(uri)?;
        let mut data = Vec::new();
        let (mut last_line, mut last_start) = (0, 0);
        for (at, token) in doc.index.tokens.iter().enumerate() {
            let Some((kind, modifiers)) = look::classify(&doc.index, at) else {
                continue;
            };
            let line = token.span.line - 1;
            let start = doc.utf16(token.span.line, token.span.col);
            let length = doc.utf16(token.span.line, token.span.end) - start;
            if length == 0 {
                continue;
            }
            let delta = if line == last_line {
                start - last_start
            } else {
                start
            };
            for value in [line - last_line, delta, length, kind, modifiers] {
                data.push(value.into());
            }
            (last_line, last_start) = (line, start);
        }
        Some(obj([("data", Json::Arr(data))]))
    }
}

fn capabilities() -> Json {
    let names = |list: &[&str]| Json::Arr(list.iter().map(|&name| name.into()).collect());
    obj([
        (
            "capabilities",
            obj([
                ("textDocumentSync", 1usize.into()),
                (
                    "completionProvider",
                    obj([("triggerCharacters", names(&[" "]))]),
                ),
                ("hoverProvider", true.into()),
                ("definitionProvider", true.into()),
                ("referencesProvider", true.into()),
                ("documentHighlightProvider", true.into()),
                ("renameProvider", true.into()),
                ("documentSymbolProvider", true.into()),
                ("documentFormattingProvider", true.into()),
                (
                    "semanticTokensProvider",
                    obj([
                        (
                            "legend",
                            obj([
                                ("tokenTypes", names(look::TYPES)),
                                ("tokenModifiers", names(look::MODIFIERS)),
                            ]),
                        ),
                        ("full", true.into()),
                    ]),
                ),
            ]),
        ),
        ("serverInfo", obj([("name", "saeromc".into())])),
    ])
}

fn real(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn standard(path: &Path) -> bool {
    crate::prescan::std_dirs()
        .iter()
        .any(|folder| path.starts_with(real(folder)))
}

fn path_of(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = (bytes[at] == b'%')
            .then(|| rest.get(at + 1..at + 3))
            .flatten()
            .and_then(|digits| u8::from_str_radix(digits, 16).ok());
        match hex {
            Some(byte) => {
                out.push(byte);
                at += 3;
            }
            None => {
                out.push(bytes[at]);
                at += 1;
            }
        }
    }
    Some(PathBuf::from(String::from_utf8_lossy(&out).into_owned()))
}

fn uri_of(path: &Path) -> String {
    let mut out = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"/-_.~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(found: Json) -> Vec<String> {
        let mut items: Vec<(String, String)> = found
            .items()
            .iter()
            .map(|item| {
                let text = |key| item.get(key).str().unwrap_or("").to_string();
                (text("sortText"), text("label"))
            })
            .collect();
        items.sort();
        items.into_iter().map(|(_, label)| label).collect()
    }

    #[test]
    fn completes_by_place() {
        let text =
            "사람은 이름이 \"새롬\"인 묶음이다.\n수들은 3과 1이다.\n수들에 4를 \n사람의 \n";
        let doc = Doc::new("file:///%EC%8B%9C%ED%97%98.sr", text);
        let at = |line: usize, typed| {
            let prefix = doc.prefix(line, usize::MAX);
            complete::complete(&doc.index, &prefix, line, doc.path.as_deref(), typed)
        };
        // 쓴 조사에 맞는 동사가 앞선다.
        let verbs = sorted(at(3, false));
        assert!(
            ["추가한다", "더한다", "곱한다"].contains(&verbs[0].as_str()),
            "{verbs:?}"
        );
        assert_eq!(at(3, true), Json::Null);
        let fields = sorted(at(4, true));
        assert_eq!(fields[0], "이름");
        assert!(fields.contains(&"길이".to_string()));
        assert_eq!(doc.path.as_deref(), Some(Path::new("/시험.sr")));
    }
}
