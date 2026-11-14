//! Reading and writing diagram files, with the Python app's checks: a file that isn't a
//! diagram is refused (it would look like an empty diagram, and Save would write over it), and
//! a file keeps its line endings.

use std::path::{Path, PathBuf};

use diagram_core::document::{Document, Kind, guess_kind, is_diagram, normalize_newlines};
use serde::Serialize;

/// A diagram read from disk.
#[derive(Debug, Serialize)]
pub struct Opened {
    /// the absolute path, as tabs compare them
    pub path: String,
    pub text: String,
    pub kind: String,
}

fn extension(path: &Path) -> Option<String> {
    path.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
}

fn name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().to_string())
}

/// The absolute form of a path (symbolic links resolved where the file exists).
pub fn absolute(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
}

/// Reads a diagram; the error is the message to show.
pub fn read(path: &Path) -> Result<Opened, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Cannot open {}:\n{e}", path.display()))?;
    let text = String::from_utf8(bytes).map_err(|e| format!("Cannot open {}:\n{e}", path.display()))?;
    let text = normalize_newlines(&text);
    let ext = extension(path);
    let kind = match ext.as_deref() {
        Some(".gsmd") => Kind::Gsm,
        Some(".erd") => Kind::Er,
        _ => guess_kind(&text),
    };
    let doc = Document::parse(&text, kind).map_err(|e| format!("Cannot read {}:\n{e}", name(path)))?;
    if !is_diagram(ext.as_deref(), &doc) {
        return Err(format!(
            "{} is not a diagram: it has no .erd or .gsmd extension and no diagram commands inside.",
            name(path)
        ));
    }
    Ok(Opened { path: absolute(path).display().to_string(), text, kind: kind.name().into() })
}

/// A diagram's extension, or a readable file with a diagram command inside (any text file
/// would load, so loading alone proves nothing).
pub fn is_diagram_file(path: &Path) -> bool {
    if matches!(extension(path).as_deref(), Some(".erd" | ".gsmd")) {
        return true;
    }
    read(path).is_ok()
}

/// The kind of diagram an existing file holds (`None`: not a diagram, or no file).
pub fn kind_of(path: &Path) -> Option<&'static str> {
    match extension(path).as_deref() {
        Some(".gsmd") => return Some("gsm"),
        Some(".erd") => return Some("er"),
        _ => {}
    }
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let any = |words: &[&str]| words.iter().any(|w| text.contains(w));
    if any(&[
        "\\abstract",
        "\\representable",
        "\\subtype",
        "\\aggregation",
        "\\grouping",
        "\\multiattribute",
        "\\specialization",
    ]) {
        Some("gsm")
    } else if any(&["\\entity", "\\relationship", "\\isa"]) {
        Some("er")
    } else {
        None
    }
}

/// Writes a diagram; an existing file with Windows line endings keeps them.
pub fn write(path: &Path, text: &str) -> Result<String, String> {
    let crlf = std::fs::read(path).is_ok_and(|old| old.windows(2).any(|w| w == b"\r\n"));
    let text = if crlf { text.replace("\r\n", "\n").replace('\n', "\r\n") } else { text.to_string() };
    std::fs::write(path, text).map_err(|e| format!("Cannot save {}:\n{e}", path.display()))?;
    Ok(absolute(path).display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_endings_and_refusals() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.erd");
        std::fs::write(&p, "\\entity{a}(0,0){A}\r\n").unwrap();
        let o = read(&p).unwrap();
        assert_eq!(o.text, "\\entity{a}(0,0){A}\n");
        write(&p, &o.text).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"\\entity{a}(0,0){A}\r\n");
        let t = dir.path().join("notes.txt");
        std::fs::write(&t, "just some text\n").unwrap();
        assert!(read(&t).unwrap_err().contains("is not a diagram"));
        assert_eq!(kind_of(&t), None);
        std::fs::write(&t, "\\abstract{a}(0,0){A}\n").unwrap();
        assert_eq!(kind_of(&t), Some("gsm"));
        assert_eq!(read(&t).unwrap().kind, "gsm");
    }

    #[test]
    fn what_counts_as_a_diagram_file() {
        // the Python app's tests/test_recent.py
        let dir = tempfile::tempdir().unwrap();
        let w = |name: &str, text: &[u8]| {
            let p = dir.path().join(name);
            std::fs::write(&p, text).unwrap();
            p
        };
        assert!(is_diagram_file(&w("c.tex", b"\\entity{a}(0,0){A}\n")));
        assert!(!is_diagram_file(&w("d.tex", b"\\section{Hello}\nText.\n")));
        assert!(!is_diagram_file(&dir.path().join("missing.txt")));
        assert!(is_diagram_file(&dir.path().join("moved_away.erd")), "listed until it's opened");
        assert!(is_diagram_file(&w("new.gsmd", b"")));
        assert!(is_diagram_file(&w("B.ERD", b"\\entity{a}(0,0){A}\n")));
        assert!(!is_diagram_file(&w("kwinrulesrc", b"[Rules]\ncount=1\n")));
        assert!(!is_diagram_file(&w("image.bin", b"\xff\xfe\x00\x01")));
        assert!(!is_diagram_file(dir.path()));
    }
}
