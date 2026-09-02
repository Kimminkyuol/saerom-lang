mod common;

#[test]
fn syntax_trees_match_frozen_goldens() {
    for path in common::sources() {
        let stem = common::stem(&path);
        let source = std::fs::read_to_string(&path).expect("소스 없음");
        let statements = saeromc::front(&source, path.parent())
            .unwrap_or_else(|found| panic!("{}", found[0].render(&source, &stem)));
        common::check("ast", "sexp", &stem, &saeromc::dump::ast(&statements));
    }
}
