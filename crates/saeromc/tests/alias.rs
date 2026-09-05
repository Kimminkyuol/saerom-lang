mod common;

const CASES: &[(&str, &str, &str)] = &[
    (
        "복사한 뒤 이어붙여도 사본은 그대로",
        "답은 \"\"이다.\n답은 답에 \"가\"를 더한 값이다.\n다른것은 답이다.\n답은 답에 \"나\"를 더한 값이다.\n\"{다른것} {답}\"을 출력한다.\n",
        "가 가나",
    ),
    (
        "묶음에 넣은 뒤 이어붙여도 넣은 값은 그대로",
        "답은 \"\"이다.\n답은 답에 \"가\"를 더한 값이다.\n것들은 묶음이다.\n것들에 답을 추가한다.\n답은 답에 \"나\"를 더한 값이다.\n\"{것들의 1번째} {답}\"을 출력한다.\n",
        "가 가나",
    ),
    (
        "복사한 값도 그대로",
        "답은 \"\"이다.\n답은 답에 \"가\"를 더한 값이다.\n사본은 답을 복사한 값이다.\n답은 답에 \"나\"를 더한 값이다.\n\"{사본} {답}\"을 출력한다.\n",
        "가 가나",
    ),
    (
        "다른 이름에서 받아온 값은 건드리지 않음",
        "씨는 \"가\"이다.\n답은 씨이다.\n답은 답에 \"나\"를 더한 값이다.\n\"{씨} {답}\"을 출력한다.\n",
        "가 가나",
    ),
    (
        "묶음에서 꺼낸 값은 건드리지 않음",
        "것은 값이 \"가\"인 묶음이다.\n답은 것의 값이다.\n답은 답에 \"나\"를 더한 값이다.\n\"{것의 값} {답}\"을 출력한다.\n",
        "가 가나",
    ),
    (
        "매개변수는 부른 쪽 값을 건드리지 않음",
        "글을 늘리다라는 것은:\n    글은 글에 \"나\"를 더한 값이다.\n    글을 출력한다.\n바탕은 \"가\"이다.\n바탕을 늘린다.\n\" {바탕}\"을 출력한다.\n",
        "가나 가",
    ),
];

#[test]
fn appending_never_changes_another_name() {
    for (index, (what, source, wanted)) in CASES.iter().enumerate() {
        let shown = common::build_and_run(source, &format!("alias{index}"));
        assert_eq!(&shown, wanted, "{what}");
    }
}

#[test]
fn appending_in_a_loop_still_works() {
    let source = "답은 \"\"이다.\n1부터 3까지 자리마다 반복한다:\n    답은 답에 \"가\"를 더한 값이다.\n답을 출력한다.\n";
    assert_eq!(common::build_and_run(source, "alias_loop"), "가가가");
}

#[test]
fn appending_a_name_to_itself_works() {
    let source = "답은 \"가\"이다.\n답은 답에 답을 더한 값이다.\n답을 출력한다.\n";
    assert_eq!(common::build_and_run(source, "alias_self"), "가가");
}
