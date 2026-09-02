use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("저장소 자리")
}

fn sources() -> Vec<PathBuf> {
    let root = root();
    let mut found: Vec<PathBuf> = std::fs::read_dir(root.join("examples"))
        .expect("examples 자리 없음")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "sr"))
        .collect();
    found.sort();
    found
}

#[test]
fn token_streams_match_frozen_goldens() {
    let root = root();
    let found = sources();
    assert!(found.len() >= 16, "검사할 소스가 모자람: {}", found.len());
    for path in found {
        let stem = path
            .file_stem()
            .expect("이름")
            .to_string_lossy()
            .into_owned();
        let golden = root.join("tests/tokens").join(format!("{stem}.tokens"));
        let wanted = std::fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("골든 없음: {}", golden.display()));
        let source = std::fs::read_to_string(&path).expect("소스 없음");
        let tokens = saeromc::tokens(&source, path.parent())
            .unwrap_or_else(|diag| panic!("{}", diag.render(&source, &stem)));
        let made = saeromc::dump::tokens(&tokens);
        assert!(
            made == wanted,
            "{stem}: 토큰열이 골든과 다름\n{}",
            first_difference(&wanted, &made)
        );
    }
}

fn first_difference(wanted: &str, made: &str) -> String {
    for (index, (left, right)) in wanted.lines().zip(made.lines()).enumerate() {
        if left != right {
            return format!("  {}줄\n  골든: {left}\n  지금: {right}", index + 1);
        }
    }
    format!(
        "  줄 수: 골든 {} / 지금 {}",
        wanted.lines().count(),
        made.lines().count()
    )
}
