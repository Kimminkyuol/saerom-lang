use saeromc::{msg, report};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fault::Usage) => {
            eprint!("{}", msg::USAGE);
            ExitCode::from(2)
        }
        Err(Fault::Message(text)) => {
            eprint!("{text}");
            ExitCode::FAILURE
        }
    }
}

/// 자리 없는 명령줄 오류를 `오류: ...` 한 줄로 꾸민다.
fn complain(message: &str) -> String {
    report::plain(msg::ERROR, message)
}

enum Fault {
    Usage,
    Message(String),
}

impl From<String> for Fault {
    fn from(text: String) -> Self {
        Fault::Message(text)
    }
}

fn run(args: &[String]) -> Result<(), Fault> {
    let mut source_path: Option<&str> = None;
    let mut output: Option<PathBuf> = None;
    let mut only_llvm = false;
    let mut only_tokens = false;
    let mut only_ast = false;
    let mut only_check = false;
    let mut only_hir = false;
    let mut only_types = false;
    let mut tuning: Option<String> = None;
    let mut frames = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--emit-llvm" => only_llvm = true,
            "--dump-tokens" => only_tokens = true,
            "--dump-ast" => only_ast = true,
            "--check" => only_check = true,
            "--dump-hir" => only_hir = true,
            "--dump-types" => only_types = true,
            "-o" => output = Some(PathBuf::from(rest.next().ok_or(Fault::Usage)?)),
            "-O" | "-O0" | "-O1" | "-O2" | "-O3" | "-Os" => tuning = Some(arg.clone()),
            "-g" => frames = true,
            "-h" | "--help" => return Err(Fault::Usage),
            other if other.starts_with('-') => return Err(Fault::Usage),
            other => source_path = Some(other),
        }
    }
    let path = source_path.ok_or(Fault::Usage)?;

    let source = std::fs::read_to_string(path)
        .map_err(|error| complain(&msg::source_unreadable(path, &error.to_string())))?;
    let base_dir = Path::new(path).parent();
    if only_tokens {
        let found = saeromc::tokens(&source, base_dir)
            .map_err(|diag| render(&[diag], &source, path))?;
        print!("{}", saeromc::dump::tokens(&found));
        return Ok(());
    }

    if only_ast {
        let statements =
            saeromc::front(&source, base_dir).map_err(|found| render(&found, &source, path))?;
        print!("{}", saeromc::dump::ast(&statements));
        return Ok(());
    }

    if only_check || only_hir || only_types {
        let (_, program) = saeromc::analyze(&source, Some(Path::new(path)))
            .map_err(|found| found.render(&source, path))?;
        if only_hir {
            print!("{}", saeromc::dump::hir(&program));
        }
        if only_types {
            print!("{}", saeromc::dump::types(&program));
        }
        return Ok(());
    }

    let triple = if only_llvm {
        String::new()
    } else {
        target_triple()
    };
    let ir = saeromc::compile(&source, Some(Path::new(path)), &triple, frames)
        .map_err(|found| found.render(&source, path))?;

    if only_llvm {
        print!("{ir}");
        return Ok(());
    }

    let output = output.unwrap_or_else(|| Path::new(path).with_extension(""));
    link(&ir, &output, tuning.as_deref()).map_err(Fault::Message)
}

fn render(found: &[saeromc::diag::Diag], source: &str, path: &str) -> String {
    let shown: Vec<String> = found.iter().map(|diag| diag.render(source, path)).collect();
    summarize(shown.join("\n"), found.len())
}

fn summarize(mut out: String, count: usize) -> String {
    if count > 1 {
        out.push_str(&report::plain(msg::ERROR, &msg::aborting(count)));
    }
    out
}

fn target_triple() -> String {
    Command::new("clang")
        .arg("-print-target-triple")
        .output()
        .ok()
        .filter(|done| done.status.success())
        .map(|done| String::from_utf8_lossy(&done.stdout).trim().to_string())
        .unwrap_or_default()
}

/// 런타임은 러스트 std 전부를 안고 오므로, 안 쓰는 조각과 심볼표를 링크에서 덜어낸다.
/// 역추적은 SR_FRAMES 표로 찍으니 DWARF 를 버려도 그대로 나온다.
#[cfg(target_os = "macos")]
const TRIM: &[&str] = &["-Wl,-dead_strip", "-Wl,-x", "-Wl,-S"];
#[cfg(not(target_os = "macos"))]
const TRIM: &[&str] = &["-Wl,--gc-sections", "-Wl,-s"];

fn link(ir: &str, output: &Path, tuning: Option<&str>) -> Result<(), String> {
    let ir_path = output.with_extension("ll");
    std::fs::write(&ir_path, ir).map_err(|error| {
        complain(&msg::write_failed(
            &ir_path.display().to_string(),
            &error.to_string(),
        ))
    })?;
    let runtime = runtime_archive()?;
    let mut clang = Command::new("clang");
    if let Some(tuning) = tuning {
        clang.arg(tuning);
    }
    let done = clang
        .args(TRIM)
        .arg(&ir_path)
        .arg(&runtime)
        .arg("-o")
        .arg(output)
        .output()
        .map_err(|error| complain(&msg::clang_missing(&error.to_string())))?;
    let _ = std::fs::remove_file(&ir_path);
    if done.status.success() {
        return Ok(());
    }
    Err(complain(&msg::link_failed(&String::from_utf8_lossy(
        &done.stderr,
    ))))
}

fn runtime_archive() -> Result<PathBuf, String> {
    const NAME: &str = "libsaerom_rt.a";
    let mut looked = Vec::new();
    if let Some(given) = std::env::var_os("SAEROM_RT") {
        looked.push(PathBuf::from(given));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(here) = exe.parent() {
            looked.push(here.join(NAME));
            looked.push(here.join("../lib/saerom").join(NAME));
        }
    }
    if let Some(found) = looked.iter().find(|path| path.exists()) {
        return Ok(found.clone());
    }
    let shown: Vec<String> = looked
        .iter()
        .map(|path| format!("  {}", path.display()))
        .collect();
    Err(complain(&msg::runtime_missing(NAME, &shown.join("\n"))))
}
