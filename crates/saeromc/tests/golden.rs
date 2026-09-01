use std::path::{Path, PathBuf};
use std::process::Command;

/// 넣는 값에 따라 출력이 달라지므로 건너뛴다.
const NEEDS_INPUT: &[&str] = &["14-입력"];

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found: Vec<PathBuf> = ["tests", "examples"]
        .iter()
        .flat_map(|folder| std::fs::read_dir(root.join(folder)).expect("자리 없음"))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "sr"))
        .filter(|path| {
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            !NEEDS_INPUT.contains(&stem.as_str())
        })
        .collect();
    found.sort();
    found
}

fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn annotations(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.split_once("# →"))
        .map(|(_, wanted)| flatten(wanted))
        .collect()
}

fn build_and_run(path: &Path) -> String {
    let output =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(path.file_stem().expect("이름 없는 파일"));
    let built = Command::new(env!("CARGO_BIN_EXE_saeromc"))
        .arg(path)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("saeromc 를 부를 수 없음");
    assert!(
        built.status.success(),
        "{} 컴파일 실패\n{}",
        path.display(),
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&output).output().expect("실행할 수 없음");
    assert!(ran.status.success(), "{} 실행 실패", path.display());
    String::from_utf8_lossy(&ran.stdout).into_owned()
}

#[test]
fn examples_print_what_they_annotate() {
    let found = fixtures();
    assert!(found.len() >= 15, "검사할 예시가 모자람: {}", found.len());
    for path in found {
        let source = std::fs::read_to_string(&path).expect("소스를 읽을 수 없음");
        let wanted = annotations(&source);
        assert!(!wanted.is_empty(), "{}: '# →' 주석이 없음", path.display());
        let printed: Vec<String> = build_and_run(&path).lines().map(flatten).collect();

        let mut left = wanted.iter();
        let mut next = left.next();
        for line in &printed {
            if next == Some(line) {
                next = left.next();
            }
        }
        assert_eq!(next, None, "{}: 출력에 없는 주석 {next:?}", path.display());
    }
}
