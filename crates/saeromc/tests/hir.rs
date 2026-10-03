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

#[test]
fn arithmetic_on_known_non_numbers_is_caught() {
    let source = "글은 \"가\"이다.\n\"{1에 \"가\"를 더한 값} {3에서 글을 뺀 값} {-참}\"을 출력한다.\n\"{글에 1을 더한 값}\"을 출력한다.\n";
    let found = saeromc::analyze(source, None)
        .err()
        .expect("오류가 나야 함");
    let messages: Vec<&str> = found
        .errors
        .iter()
        .map(|error| error.msg.as_str())
        .collect();
    assert_eq!(
        messages,
        [
            "'더하다'의 인자가 수가 아님: 문자열 \"가\"",
            "'빼다'의 인자가 수가 아님: 문자열",
            "'빼다'의 인자가 수가 아님: 논리값 참",
        ]
    );
}
