#![allow(dead_code)]

// 골든 견주기. `tests/`는 저장소에 없으므로 (gitignore) 골든이 없으면
// 첫 실행 때 지금 결과를 적어 둔다. SAEROM_BLESS=1 이면 있어도 다시 적는다.

use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("저장소 자리")
}

pub fn sources() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root().join("examples"))
        .expect("examples 자리 없음")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "sr"))
        .collect();
    found.sort();
    assert!(found.len() >= 16, "검사할 소스가 모자람: {}", found.len());
    found
}

pub fn stem(path: &Path) -> String {
    path.file_stem()
        .expect("식별자")
        .to_string_lossy()
        .into_owned()
}

/// `tests/<kind>/<stem>.<ext>` 와 견준다.
pub fn check(kind: &str, ext: &str, stem: &str, made: &str) {
    let golden = root()
        .join("tests")
        .join(kind)
        .join(format!("{stem}.{ext}"));
    let wanted = std::fs::read_to_string(&golden).ok();
    if wanted.is_none() || std::env::var_os("SAEROM_BLESS").is_some() {
        std::fs::create_dir_all(golden.parent().expect("자리")).expect("자리를 만들 수 없음");
        std::fs::write(&golden, made).expect("골든을 쓸 수 없음");
        eprintln!("골든 적음: {}", golden.display());
        return;
    }
    let wanted = wanted.expect("골든");
    assert!(
        made == wanted,
        "{stem}: {kind} 골든과 다름 (고치려면 SAEROM_BLESS=1)\n{}",
        gap(&wanted, made)
    );
}

fn gap(wanted: &str, made: &str) -> String {
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
