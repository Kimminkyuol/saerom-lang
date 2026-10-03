mod common;

const CASES: &[(&str, &str, &str)] = &[
    (
        "함수 안에서 바깥 이름을 고침",
        "셈은 0이다.\n늘리다라는 것은:\n    셈은 셈에 1을 더한 값이다.\n늘린다.\n늘린다.\n\"{셈}\"을 출력한다.\n",
        "2",
    ),
    (
        "뒤에서 매긴 바깥 이름도 고침",
        "켜다라는 것은:\n    상태는 참이다.\n상태는 거짓이다.\n켠다.\n\"{상태}\"을 출력한다.\n",
        "참",
    ),
    (
        "반복 변수는 지역",
        "자리는 9이다.\n돌다라는 것은:\n    1부터 2까지 자리마다 반복한다:\n        \"{자리} \"를 출력한다.\n돈다.\n\"{자리}\"을 출력한다.\n",
        "1 2 9",
    ),
    (
        "매개변수는 지역",
        "수는 9이다.\n수를 비우다라는 것은:\n    수는 0이다.\n    수를 반환한다.\n\"{1을 비운 값} {수}\"을 출력한다.\n",
        "0 9",
    ),
    (
        "바깥에 없는 이름은 지역",
        "쓰다듬다라는 것은:\n    털은 1이다.\n    털을 반환한다.\n\"{쓰다듬은 값}\"을 출력한다.\n",
        "1",
    ),
];

#[test]
fn assignment_reaches_outer_names() {
    for (index, (what, source, wanted)) in CASES.iter().enumerate() {
        let shown = common::build_and_run(source, &format!("scope{index}"));
        assert_eq!(&shown, wanted, "{what}");
    }
}
