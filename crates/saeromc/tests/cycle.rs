mod common;

const MADE: &str = "상자는 묶음이다.\n상자의 안은 상자이다.\n";

#[test]
fn cyclic_table_prints() {
    let source = format!("{MADE}\"{{상자}}\"을 출력한다.\n");
    assert_eq!(common::build_and_run(&source, "cycle_print"), "{안: {…}}");
}

#[test]
fn cyclic_table_copies_its_shape() {
    let source = format!(
        "{MADE}사본은 상자를 복사한 값이다.\n사본의 표는 1이다.\n\"{{사본의 안의 안의 표}} {{상자}}\"을 출력한다.\n"
    );
    assert_eq!(common::build_and_run(&source, "cycle_copy"), "1 {안: {…}}");
}

#[test]
fn cyclic_tables_compare() {
    let source = format!(
        "{MADE}다른것은 묶음이다.\n다른것의 안은 다른것이다.\n\"{{상자가 다른것과 같은지}} {{상자가 상자와 같은지}}\"을 출력한다.\n"
    );
    assert_eq!(common::build_and_run(&source, "cycle_equal"), "참 참");
}
