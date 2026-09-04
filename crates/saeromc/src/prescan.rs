use crate::diag::{Diag, Result};
use crate::hangul::{is_syllable, Ending};
use crate::lex::{tokenize, Tok, Token, Vocabulary};
use crate::msg;
use crate::sig::{ordered, Marker, Signatures};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn prescan(source: &str, base_dir: Option<&Path>) -> Result<Vocabulary> {
    scan(source, base_dir, &mut HashSet::new())
}

fn scan(
    source: &str,
    base_dir: Option<&Path>,
    chain: &mut HashSet<PathBuf>,
) -> Result<Vocabulary> {
    let tokens = tokenize(source, &Vocabulary::default())?;
    let mut stems = declared_stems(&tokens);
    stems.extend(imported_stems(&tokens, base_dir, chain));
    Ok(Vocabulary::new(declared_names(&tokens), stems))
}

fn declared_names(tokens: &[Token]) -> HashSet<String> {
    let mut names = HashSet::new();
    for (index, token) in tokens.iter().enumerate() {
        let Tok::Name(name) = &token.tok else {
            continue;
        };
        match tokens.get(index + 1).map(|t| &t.tok) {
            Some(Tok::Particle { .. }) => {
                names.insert(name.clone());
            }
            Some(Tok::Symbol(':')) if opens_entry(tokens, index) => {
                names.insert(name.clone());
            }
            _ => {}
        }
    }
    names
}

fn opens_entry(tokens: &[Token], index: usize) -> bool {
    index > 0 && matches!(tokens[index - 1].tok, Tok::Symbol('{') | Tok::Symbol(','))
}

fn declared_stems(tokens: &[Token]) -> HashSet<String> {
    let mut stems = HashSet::new();
    for window in tokens.windows(5) {
        let [head, quotative, thing, topic, colon] = window else {
            continue;
        };
        let Tok::Name(name) = &head.tok else { continue };
        if !dictionary_form(name) {
            continue;
        }
        let quoted = matches!(
            quotative.tok,
            Tok::Copula {
                ending: Ending::Quotative
            }
        );
        let is_thing = matches!(&thing.tok, Tok::Name(word) if word == "것");
        let is_topic = matches!(topic.tok, Tok::Particle { role: "topic", .. });
        if quoted && is_thing && is_topic && colon.tok == Tok::Symbol(':') {
            let mut stem = name.clone();
            stem.pop();
            stems.insert(stem);
        }
    }
    stems
}

fn dictionary_form(name: &str) -> bool {
    let mut chars = name.chars().rev();
    chars.next() == Some('다')
        && chars.next().is_some_and(is_syllable)
        && !name.ends_with("하다")
        && !name.ends_with("되다")
        && !name.ends_with("이다")
}

fn imported_stems(
    tokens: &[Token],
    base_dir: Option<&Path>,
    chain: &mut HashSet<PathBuf>,
) -> HashSet<String> {
    let mut stems = HashSet::new();
    for found in imports(tokens) {
        let Some(path) = resolve_module(&found.module, base_dir) else {
            continue;
        };
        if !chain.insert(path.clone()) {
            continue;
        }
        if let Ok(source) = std::fs::read_to_string(&path) {
            if let Ok(vocab) = scan(&source, path.parent(), chain) {
                stems.extend(vocab.stems);
            }
        }
        chain.remove(&path);
    }
    stems
}

pub fn resolve_module(name: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    let wanted = crate::hangul::to_nfc(&format!("{name}.sr"));
    let found = find_in(base_dir?, &wanted)?;
    Some(found.canonicalize().unwrap_or(found))
}

fn find_in(folder: &Path, wanted: &str) -> Option<PathBuf> {
    let direct = folder.join(wanted);
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(folder).ok()?.flatten().find_map(|entry| {
        let name = entry.file_name();
        (crate::hangul::to_nfc(&name.to_string_lossy()) == wanted).then(|| entry.path())
    })
}

#[derive(Default, Clone)]
pub struct Program {
    pub vocab: Vocabulary,
    pub signatures: Signatures,
    pub modules: HashSet<String>,
    pub nouns: HashSet<String>,
}

pub fn survey(source: &str, base_dir: Option<&Path>) -> Result<Program> {
    let mut chain = HashSet::new();
    let mut program = Program {
        signatures: Signatures::builtin(),
        ..Program::default()
    };
    gather(source, base_dir, &mut chain, &mut program, None, true)?;
    Ok(program)
}

fn gather(
    source: &str,
    base_dir: Option<&Path>,
    chain: &mut HashSet<PathBuf>,
    into: &mut Program,
    taking: Option<&[String]>,
    root: bool,
) -> Result<()> {
    let vocab = scan(source, base_dir, chain)?;
    let tokens = tokenize(source, &vocab)?;
    let mine = definitions(&tokens);
    let wanted = |name: &str| taking.is_none_or(|names| names.iter().any(|n| n == name));

    for (head, params) in &mine {
        if !wanted(head) {
            continue;
        }
        if head.ends_with('다') {
            into.signatures
                .add(head, ordered(params.iter().map(|&(marker, _)| marker)));
        } else {
            into.nouns.insert(head.clone());
        }
    }
    if root {
        shadowed(&vocab, &tokens)?;
        into.vocab = vocab;
    }

    for import in imports(&tokens) {
        let Some(path) = resolve_module(&import.module, base_dir) else {
            continue;
        };
        if !chain.insert(path.clone()) {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            gather(
                &text,
                path.parent(),
                chain,
                into,
                import.names.as_deref(),
                false,
            )?;
        }
        chain.remove(&path);
        if import.names.is_none() {
            into.modules.insert(import.module);
        }
    }
    Ok(())
}

fn shadowed(vocab: &Vocabulary, tokens: &[Token]) -> Result<()> {
    for token in tokens {
        let Tok::Name(name) = &token.tok else { continue };
        if let Some(verb) = vocab.verb_named(name) {
            return Err(Diag::name(
                msg::name_shadows_verb(name, verb),
                token.span,
            ));
        }
    }
    Ok(())
}

fn definitions(tokens: &[Token]) -> Vec<(String, Vec<(Marker, String)>)> {
    let mut found = Vec::new();
    for line in lines(tokens) {
        let Some((head, params)) = definition_head(&line) else {
            continue;
        };
        found.push((head, params));
    }
    found
}

fn definition_head(line: &[&Tok]) -> Option<(String, Vec<(Marker, String)>)> {
    let (front, tail) = line.split_at(line.len().checked_sub(5)?);
    let [Tok::Name(head), Tok::Copula {
        ending: Ending::Quotative,
    }, Tok::Name(thing), Tok::Particle { role: "topic", .. }, Tok::Symbol(':')] = tail
    else {
        return None;
    };
    if thing != "것" {
        return None;
    }
    let mut params = Vec::new();
    for pair in front.chunks(2) {
        let [Tok::Name(name), Tok::Particle { canon, .. }] = pair else {
            return None;
        };
        params.push((Marker::Case(canon), name.clone()));
    }
    Some((head.clone(), params))
}

struct Import {
    module: String,
    names: Option<Vec<String>>,
}

fn imports(tokens: &[Token]) -> Vec<Import> {
    let mut found = Vec::new();
    for line in lines(tokens) {
        let [Tok::Name(module), Tok::Particle { canon, .. }, rest @ ..] = line.as_slice()
        else {
            continue;
        };
        if !matches!(rest.last(), Some(Tok::Symbol('.'))) {
            continue;
        }
        if !matches!(rest.iter().rev().nth(1), Some(Tok::Verb { name, .. }) if name == "가져오다")
        {
            continue;
        }
        let names = (*canon == "에서").then(|| {
            rest.iter()
                .filter_map(|tok| match tok {
                    Tok::Name(name) => Some(name.clone()),
                    _ => None,
                })
                .collect()
        });
        found.push(Import {
            module: module.clone(),
            names,
        });
    }
    found
}

fn lines(tokens: &[Token]) -> Vec<Vec<&Tok>> {
    let mut all = Vec::new();
    let mut line = Vec::new();
    for token in tokens {
        match &token.tok {
            Tok::Indent(_) | Tok::Dedent(_) => {}
            Tok::Newline => all.push(std::mem::take(&mut line)),
            other => line.push(other),
        }
    }
    all
}
