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
fn resolved_trees_match_frozen_goldens() {
    let root = root();
    std::env::set_var("SAEROM_STD", root.join("std"));
    for path in sources(&root) {
        let stem = path
            .file_stem()
            .expect("이름")
            .to_string_lossy()
            .into_owned();
        let golden = root.join("tests/hir").join(format!("{stem}.hir"));
        let wanted = std::fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("골든 없음: {}", golden.display()));
        let source = std::fs::read_to_string(&path).expect("소스 없음");
        let (_, program) = saeromc::analyze(&source, Some(&path))
            .unwrap_or_else(|found| panic!("{}", found.render(&source, &stem)));
        let made = saeromc::dump::hir(&program);
        assert!(
            made == wanted,
            "{stem}: 해석 결과가 골든과 다름\n{}",
            first_gap(&wanted, &made)
        );
    }
}

#[test]
fn unknown_verbs_and_particles_are_caught() {
    let source = "수는 3이다.\n수를 5로 자랑한다.\n수에 5를 나눈다.\n없는이름을 출력한다.\n";
    let found = saeromc::analyze(source, None)
        .err()
        .expect("오류가 나야 함");
    let messages: Vec<&str> = found
        .errors
        .iter()
        .map(|error| error.msg.as_str())
        .collect();
    assert_eq!(messages.len(), 3, "{messages:?}");
    assert!(
        messages[0].contains("'자랑하다' 정의되지 않음"),
        "{messages:?}"
    );
    assert!(messages[1].contains("'나누다'를 조사"), "{messages:?}");
    assert!(
        messages[2].contains("'없는이름' 정의되지 않음"),
        "{messages:?}"
    );
}

fn first_gap(wanted: &str, made: &str) -> String {
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
