mod common;

#[test]
fn resolved_trees_match_frozen_goldens() {
    for path in common::sources() {
        let stem = common::stem(&path);
        let source = std::fs::read_to_string(&path).expect("소스 없음");
        let (_, program) = saeromc::analyze(&source, Some(&path))
            .unwrap_or_else(|found| panic!("{}", found.render(&source, &stem)));
        common::check("hir", "hir", &stem, &saeromc::dump::hir(&program));
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
