//! The check harness for arranging (the root README, "Arranging a diagram automatically"): the core's
//! half, run by `spike/arrange_check.sh` with the page's layout (`lib/arrange.ts`) in Node.js in
//! between.
//!
//!     cargo run -q -p diagram-core --features json --example arrange_check -- graph FILE > graph.json
//!     cargo run -q -p diagram-core --features json --example arrange_check -- best FILE CANDIDATES.json OUT
//!
//! `best` measures every candidate on the drawing, writes the best one into OUT and prints the figures
//! (the original's, each candidate's, which was chosen) as JSON.

use diagram_core::arrange::{self, json};
use diagram_core::document::{Document, Kind};
use diagram_core::style::LabelLanguage;

fn load(path: &str) -> Document {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let kind = if path.ends_with(".gsmd") { Kind::Gsm } else { Kind::Er };
    Document::parse(&text, kind).unwrap_or_else(|_| Document::parse("", kind).expect("empty document"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let lang = LabelLanguage::English;
    match (args.get(1).map(String::as_str), args.get(2)) {
        (Some("graph"), Some(file)) => println!("{}", json::graph(&arrange::graph(&load(file), lang))),
        (Some("best"), Some(file)) => {
            let doc = load(file);
            let cands: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&args[3]).unwrap_or_default())
                    .unwrap_or_default();
            let original = arrange::figures(&doc, lang, None);
            let mut rows =
                vec![format!("{{\"name\":\"original\",\"figures\":{}}}", json::figures(&original))];
            let mut best: Option<(f64, arrange::Arrangement, String)> = None;
            for c in cands.as_array().cloned().unwrap_or_default() {
                let name = c["name"].as_str().unwrap_or("").to_string();
                let Ok(a) = json::arrangement(&c["arrangement"].to_string()) else { continue };
                let f = arrange::try_arrangement(&doc, &a, lang);
                rows.push(format!(
                    "{{\"name\":{},\"figures\":{}}}",
                    serde_json::json!(name),
                    json::figures(&f)
                ));
                if best.as_ref().is_none_or(|b| f.score() < b.0) {
                    best = Some((f.score(), a, name));
                }
            }
            if let Some((_, a, name)) = best {
                let mut d = doc.clone();
                arrange::apply(&mut d, &a, lang);
                let text = d.to_text();
                // the file stays valid: it loads again and saves back unchanged
                match diagram_core::document::Document::parse(&text, d.kind) {
                    Ok(again) if again.to_text() == text => {}
                    Ok(_) => eprintln!("NOT SAVED BACK UNCHANGED after a reload"),
                    Err(e) => eprintln!("DOES NOT LOAD AGAIN: {e:?}"),
                }
                std::fs::write(&args[4], text).expect("write");
                println!("{{\"rows\":[{}],\"chosen\":{}}}", rows.join(","), serde_json::json!(name));
            }
        }
        _ => eprintln!("arrange_check graph FILE | best FILE CANDIDATES.json OUT"),
    }
}
