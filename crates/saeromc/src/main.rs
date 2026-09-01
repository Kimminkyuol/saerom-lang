use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
새롬 컴파일러

  saeromc <파일.sr> [-o <출력>]   실행파일로 컴파일
  saeromc --emit-llvm <파일.sr>   LLVM IR만 찍는다
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
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--emit-llvm" => only_llvm = true,
            "-o" => output = Some(PathBuf::from(rest.next().ok_or(Fault::Usage)?)),
            "-h" | "--help" => return Err(Fault::Usage),
            other if other.starts_with('-') => return Err(Fault::Usage),
            other => source_path = Some(other),
        }
    }
    let path = source_path.ok_or(Fault::Usage)?;

    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("파일을 읽을 수 없음: {path} ({error})\n"))?;
    let triple = if only_llvm {
        String::new()
    } else {
        target_triple()
    };
    let ir = saeromc::compile(&source, &triple).map_err(|diag| diag.render(&source, path))?;

    if only_llvm {
        print!("{ir}");
        return Ok(());
    }

    let output = output.unwrap_or_else(|| Path::new(path).with_extension(""));
    link(&ir, &output).map_err(Fault::Message)
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

fn link(ir: &str, output: &Path) -> Result<(), String> {
    let ir_path = output.with_extension("ll");
    std::fs::write(&ir_path, ir)
        .map_err(|error| format!("{}를 쓸 수 없음: {error}\n", ir_path.display()))?;
    let runtime = runtime_archive()?;
    let done = Command::new("clang")
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
