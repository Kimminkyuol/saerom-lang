use crate::diag::Result;
use crate::hangul::{is_syllable, Ending};
use crate::lex::{tokenize, Tok, Token, Vocabulary};
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
    Ok(Vocabulary {
        names: declared_names(&tokens),
        stems,
    })
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

fn imported_modules(tokens: &[Token]) -> Vec<String> {
    let mut found = Vec::new();
    let mut line: Vec<&Tok> = Vec::new();
    for token in tokens {
        match &token.tok {
            Tok::Indent(_) | Tok::Dedent(_) => {}
            Tok::Newline => {
                if let [Tok::Name(name), .., Tok::Verb { name: verb, .. }, Tok::Symbol('.')] =
                    line.as_slice()
                {
                    if verb == "가져오다" {
                        found.push(name.clone());
                    }
                }
                line.clear();
            }
            other => line.push(other),
        }
    }
    found
}

fn imported_stems(
    tokens: &[Token],
    base_dir: Option<&Path>,
    chain: &mut HashSet<PathBuf>,
) -> HashSet<String> {
    let mut stems = HashSet::new();
    for name in imported_modules(tokens) {
        let Some(path) = resolve_module(&name, base_dir) else {
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
    base_dir
        .into_iter()
        .map(Path::to_path_buf)
        .chain(standard_library())
        .find_map(|folder| find_in(&folder, &wanted))
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

fn standard_library() -> Option<PathBuf> {
    if let Ok(set) = std::env::var("SAEROM_STD") {
        return Some(PathBuf::from(set));
    }
    let exe = std::env::current_exe().ok()?;
    let found = exe.parent()?.parent()?.parent()?.join("std");
    found.is_dir().then_some(found)
}
