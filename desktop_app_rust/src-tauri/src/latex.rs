//! pdflatex for the preview and the exports (from `preview.py`): a job in a folder of its own,
//! with the packages bundled with the app, so that the result doesn't depend on which version the
//! TeX installation has; everything else (fonts, the preamble's packages) comes from it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use diagram_core::document::Kind;
use diagram_core::export::{job_file, parse_errors, wrapper};

pub const ERDIAGRAM: &str = include_str!("../../../latex_packages/erdiagram.sty");
pub const GSMDIAGRAM: &str = include_str!("../../../latex_packages/gsmdiagram.sty");
const TIMEOUT: Duration = Duration::from_secs(60);

/// What to install, said where pdflatex is missing.
pub const TEX_HINT: &str = if cfg!(windows) {
    "MiKTeX or TeX Live (and make sure pdflatex is on the PATH)"
} else {
    "TeX Live (e.g. 'sudo apt install texlive-latex-extra' or 'sudo dnf install texlive-scheme-medium')"
};

/// A program on the PATH.
pub fn which(program: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{program}.exe") } else { program.to_string() };
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(&exe)).find(|p| p.is_file())
}

/// What the preview and exports ask for.
#[derive(Clone, Copy)]
pub struct Job<'a> {
    pub text: &'a str,
    pub kind: Kind,
    pub preamble: &'a str,
    pub language: &'a str,
    pub grid: bool,
}

/// The PDF, or the errors to show (`line N: message` for the diagram itself).
pub fn compile(job: &Job<'_>) -> Result<Vec<u8>, Vec<String>> {
    let Some(exe) = which("pdflatex") else {
        return Err(vec![format!("pdflatex was not found. Please install {TEX_HINT}.")]);
    };
    let dir = tempfile::Builder::new().prefix("erdtex-").tempdir().map_err(|e| vec![e.to_string()])?;
    let diagram = job_file(job.kind);
    let write =
        |name: &str, text: &str| std::fs::write(dir.path().join(name), text).map_err(|e| vec![e.to_string()]);
    write(diagram, job.text)?;
    write("preview.tex", &wrapper(job.kind, diagram, job.preamble, job.language, job.grid))?;
    // the packages next to the job (TEXINPUTS alone is not honoured by every TeX)
    write("erdiagram.sty", ERDIAGRAM)?;
    write("gsmdiagram.sty", GSMDIAGRAM)?;
    run(&exe, dir.path())?;
    let log = std::fs::read(dir.path().join("preview.log")).map(|b| String::from_utf8_lossy(&b).into_owned());
    let errors = parse_errors(&log.unwrap_or_default(), diagram);
    match std::fs::read(dir.path().join("preview.pdf")) {
        Ok(pdf) if errors.is_empty() => Ok(pdf),
        _ if errors.is_empty() => Err(vec!["pdflatex failed".into()]),
        _ => Err(errors),
    }
}

fn run(exe: &Path, dir: &Path) -> Result<(), Vec<String>> {
    let mut child = Command::new(exe)
        .args(["-interaction=nonstopmode", "-halt-on-error", "-file-line-error", "preview.tex"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| vec![format!("pdflatex could not be started: {e}")])?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if start.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(vec!["pdflatex took too long".into()]);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(vec![e.to_string()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(text: &str) -> Job<'_> {
        Job {
            text,
            kind: Kind::Er,
            preamble: diagram_core::export::DEFAULT_PREAMBLE,
            language: "english",
            grid: false,
        }
    }

    #[test]
    fn compiles_and_reports_errors_with_their_line() {
        if which("pdflatex").is_none() {
            eprintln!("no pdflatex: skipped");
            return;
        }
        let pdf = compile(&job("\\entity{a}(0,0){A}\n")).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        let errors = compile(&job("\\entity{a}(0,0){A \\undefinedmacro}\n")).unwrap_err();
        assert!(errors[0].starts_with("line 1: "), "{errors:?}");
        let gsm = Job { kind: Kind::Gsm, grid: true, ..job("\\abstract{a}(0,0){A}\n") };
        assert!(compile(&gsm).is_ok());
    }
}
