//! Every diagram file in the repository loads and is written back byte for byte.

use std::path::{Path, PathBuf};

use diagram_core::document::{Document, kind_for, normalize_newlines};

fn diagram_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let skip = [".git", "build", "dist", "target", "node_modules"];
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !skip.contains(&name.as_str()) {
                diagram_files(&path, out);
            }
        } else if name.ends_with(".erd") || name.ends_with(".gsmd") {
            out.push(path);
        }
    }
}

#[test]
fn all_repository_files_round_trip() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut files = Vec::new();
    diagram_files(&repo, &mut files);
    assert!(files.len() > 10, "found only {} files", files.len());
    for f in files {
        let text = normalize_newlines(&std::fs::read_to_string(&f).unwrap());
        let ext = f.extension().map(|e| format!(".{}", e.to_string_lossy()));
        let doc = Document::parse(&text, kind_for(ext.as_deref(), &text))
            .unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        assert_eq!(doc.to_text(), text, "{}", f.display());
        assert!(!doc.modified());
    }
}
