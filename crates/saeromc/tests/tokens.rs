mod common;

#[test]
fn token_streams_match_frozen_goldens() {
    for path in common::sources() {
        let stem = common::stem(&path);
        let source = std::fs::read_to_string(&path).expect("소스 없음");
        let tokens = saeromc::tokens(&source, path.parent())
            .unwrap_or_else(|diag| panic!("{}", diag.render(&source, &stem)));
        common::check("tokens", "tokens", &stem, &saeromc::dump::tokens(&tokens));
    }
}
