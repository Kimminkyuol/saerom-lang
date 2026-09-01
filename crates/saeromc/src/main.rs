use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
새롬

  saeromc <파일.sr> [-o <출력>]
  saeromc --emit-llvm <파일.sr>
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fault::Usage) => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
        Err(Fault::Message(text)) => {
            eprint!("{text}");
            ExitCode::FAILURE
        }
    }
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
    let mut tuning: Option<String> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--emit-llvm" => only_llvm = true,
            "--dump-tokens" => only_tokens = true,
            "--dump-ast" => only_ast = true,
            "--check" => only_check = true,
            "--dump-hir" => only_hir = true,
            "-o" => output = Some(PathBuf::from(rest.next().ok_or(Fault::Usage)?)),
            "-O" | "-O0" | "-O1" | "-O2" | "-O3" | "-Os" => tuning = Some(arg.clone()),
            "-h" | "--help" => return Err(Fault::Usage),
            other if other.starts_with('-') => return Err(Fault::Usage),
            other => source_path = Some(other),
        }
    }
    let path = source_path.ok_or(Fault::Usage)?;

    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("파일을 읽을 수 없음: {path} ({error})\n"))?;
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

    if only_check || only_hir {
        let (_, program) = saeromc::analyze(&source, Some(Path::new(path)))
            .map_err(|found| found.render(&source, path))?;
        if only_hir {
            print!("{}", saeromc::dump::hir(&program));
        }
        return Ok(());
    }

    let triple = if only_llvm {
        String::new()
    } else {
        target_triple()
    };
    let ir = saeromc::compile(&source, Some(Path::new(path)), &triple)
        .map_err(|found| found.render(&source, path))?;

    if only_llvm {
        print!("{ir}");
        return Ok(());
    }

    let output = output.unwrap_or_else(|| Path::new(path).with_extension(""));
    link(&ir, &output, tuning.as_deref()).map_err(Fault::Message)
}

fn render(found: &[saeromc::diag::Diag], source: &str, path: &str) -> String {
    found
        .iter()
        .map(|diag| diag.render(source, path))
        .collect::<Vec<_>>()
        .join("\n")
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

fn link(ir: &str, output: &Path, tuning: Option<&str>) -> Result<(), String> {
    let ir_path = output.with_extension("ll");
    std::fs::write(&ir_path, ir)
        .map_err(|error| format!("{}를 쓸 수 없음: {error}\n", ir_path.display()))?;
    let runtime = runtime_archive()?;
    let mut clang = Command::new("clang");
    if let Some(tuning) = tuning {
        clang.arg(tuning);
    }
    let done = clang
        .arg(&ir_path)
        .arg(&runtime)
        .arg("-o")
        .arg(output)
        .output()
        .map_err(|error| format!("clang을 부를 수 없음: {error}\n"))?;
    let _ = std::fs::remove_file(&ir_path);
    if done.status.success() {
        return Ok(());
    }
    Err(format!(
        "링크 실패\n{}",
        String::from_utf8_lossy(&done.stderr)
    ))
}

fn runtime_archive() -> Result<PathBuf, String> {
    let here = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .ok_or("컴파일러 자리를 알 수 없음\n")?;
    let found = here.join("libsaerom_rt.a");
    if found.exists() {
        return Ok(found);
    }
    Err(format!("런타임을 찾을 수 없음: {}\n", found.display()))
}
