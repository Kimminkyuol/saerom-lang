use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("저장소 자리")
}

fn sources(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = ["examples", "std"]
        .iter()
        .flat_map(|folder| std::fs::read_dir(root.join(folder)).expect("자리 없음"))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "sr"))
        .collect();
    found.sort();
    found
}

#[test]
fn syntax_trees_match_frozen_goldens() {
    let root = root();
    std::env::set_var("SAEROM_STD", root.join("std"));
    let found = sources(&root);
    assert!(found.len() >= 18, "검사할 소스가 모자람: {}", found.len());
    for path in found {
        let stem = path
            .file_stem()
            .expect("이름")
            .to_string_lossy()
            .into_owned();
        let golden = root.join("tests/ast").join(format!("{stem}.sexp"));
        let wanted = std::fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("골든 없음: {}", golden.display()));
        let source = std::fs::read_to_string(&path).expect("소스 없음");
        let statements = saeromc::front(&source, path.parent())
            .unwrap_or_else(|found| panic!("{}", found[0].render(&source, &stem)));
        let made = saeromc::dump::ast(&statements);
        assert!(
            made == wanted,
            "{stem}: 구문트리가 골든과 다름\n{}",
            difference(&wanted, &made)
        );
    }
}

fn difference(wanted: &str, made: &str) -> String {
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
