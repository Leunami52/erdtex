//! The style keys of erdiagram.sty / gsmdiagram.sty and their defaults (keep in sync with the
//! packages: a test in the Python app compiles every key).  A diagram's `\setup{...}` changes
//! them for that diagram.

use crate::document::Kind;
use crate::latex_text::{Fmt, font_switches, parse_length_pt};
use crate::model::Options;
use crate::text::parse_float;

/// The language of the keyword labels (IS_A, CAN_BE, PART_OF): a setting of the app, not of
/// the file.  In a LaTeX document, the document's language decides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelLanguage {
    #[default]
    English,
    German,
}

/// The keyword labels in German.
pub const GERMAN_LABELS: &[(&str, &str)] =
    &[("isa label", "IST\\_EIN*E"), ("canbe label", "KANN\\_SEIN"), ("partof label", "TEIL\\_VON")];

pub const ER_DEFAULTS: &[(&str, &str)] = &[
    ("entity fill", "gray!15"),
    ("entity draw", "black"),
    ("entity line width", "0.6pt"),
    ("entity text", "black"),
    ("entity font", "\\bfseries"),
    ("entity minimum width", "2.8cm"),
    ("entity minimum height", "1cm"),
    ("entity inner sep", "5pt"),
    ("weak double distance", "2pt"),
    ("rel fill", "white"),
    ("rel draw", "black"),
    ("rel line width", "0.6pt"),
    ("rel text", "black"),
    ("rel font", "\\itshape"),
    ("rel minimum width", "2cm"),
    ("rel minimum height", "1.4cm"),
    ("rel aspect", "1.6"),
    ("dependency color", "red!70!black"),
    ("dependency font", "\\normalfont\\bfseries"),
    ("isa label", "IS\\_A"),
    ("canbe label", "CAN\\_BE"),
    ("partof label", "PART\\_OF"),
    ("label language", "auto"),
    ("isa shape", "hexagon"),
    ("isa minimum width", "2cm"),
    ("isa minimum height", "1cm"),
    ("isa slant", "0.5"),
    ("constraint style", "letters"),
    ("constraint color", "red!70!black"),
    ("constraint font", "\\normalfont\\small"),
    ("constraint pos", "inside"),
    ("attr font", "\\small"),
    ("attr text", "black"),
    ("attr spread", "0.35cm"),
    ("attr gap", ""),
    ("attr tick", "0.3cm"),
    ("attr stem", "0.3cm"),
    ("attr indent", "3pt"),
    ("attribute style", "compact"),
    ("chen dist", "0.9cm"),
    ("card notation", "n"),
    ("card symbols", "labels"),
    ("crow length", "6pt"),
    ("crow width", "3.5pt"),
    ("show types", "false"),
    ("type sep", ":\\ "),
    ("type font", ""),
    ("attribute fill", "white"),
    ("attribute draw", "black"),
    ("attribute minimum width", "1.2cm"),
    ("attribute minimum height", "0.7cm"),
    ("card font", "\\footnotesize"),
    ("card text", "black"),
    ("card placement", "inline"),
    ("card dist", "0.3cm"),
    ("card side", "auto"),
    ("role font", "\\footnotesize\\itshape"),
    ("role text", "black"),
    ("role pos", "auto"),
    ("edge color", "black"),
    ("edge line width", "0.5pt"),
    ("link double distance", "1.5pt"),
    ("note color", "red!70!black"),
    ("note font", "\\small"),
    ("subdomain color", "blue!60"),
    ("subdomain label font", "\\small\\bfseries"),
    ("subdomain line width", "1.2pt"),
    ("subdomain tuck", "4pt"),
    ("subdomain label vpad", "4pt"),
    ("subdomain label hpad", "5pt"),
    ("rule warnings", "true"),
    ("grid color", "gray!25"),
    ("grid label color", "gray!55"),
    ("sans serif", "false"),
];

pub const GSM_DEFAULTS: &[(&str, &str)] = &[
    ("abstract fill", "gray!15"),
    ("abstract draw", "black"),
    ("abstract line width", "0.6pt"),
    ("abstract text", "black"),
    ("abstract font", "\\bfseries"),
    ("abstract minimum width", "2.6cm"),
    ("abstract minimum height", "0.6cm"),
    ("abstract apex angle", "90"),
    ("repr fill", "gray!15"),
    ("repr draw", "black"),
    ("repr line width", "0.6pt"),
    ("repr text", "black"),
    ("repr font", "\\bfseries"),
    ("repr minimum width", "2.4cm"),
    ("repr minimum height", "0.9cm"),
    ("repr corner radius", "6pt"),
    ("sub fill", "gray!15"),
    ("sub draw", "black"),
    ("sub line width", "0.6pt"),
    ("sub text", "black"),
    ("sub font", "\\bfseries"),
    ("sub minimum size", "1.4cm"),
    ("aggr fill", "white"),
    ("aggr draw", "black"),
    ("aggr line width", "0.6pt"),
    ("aggr size", "0.8cm"),
    ("aggr label color", "red!70!black"),
    ("aggr label font", "\\small"),
    ("aggr label pos", "auto"),
    ("edge color", "black"),
    ("edge line width", "0.5pt"),
    ("arrow length", "6pt"),
    ("arrow width", "4pt"),
    ("generalization line width", "2pt"),
    ("specialization double distance", "1.5pt"),
    ("label font", "\\footnotesize\\itshape"),
    ("label color", "black"),
    ("label pos", "0.5"),
    ("label side", "auto"),
    ("label sloped", "true"),
    ("note color", "red!70!black"),
    ("note font", "\\small"),
    ("subdomain color", "blue!60"),
    ("subdomain label font", "\\small\\bfseries"),
    ("subdomain line width", "1.2pt"),
    ("subdomain tuck", "4pt"),
    ("subdomain label vpad", "4pt"),
    ("subdomain label hpad", "5pt"),
    ("rule warnings", "true"),
    ("grid color", "gray!25"),
    ("grid label color", "gray!55"),
    ("sans serif", "false"),
];

/// The defaults of a kind of diagram.
pub fn defaults(kind: Kind) -> &'static [(&'static str, &'static str)] {
    match kind {
        Kind::Er => ER_DEFAULTS,
        Kind::Gsm => GSM_DEFAULTS,
    }
}

/// The style of one diagram: the packages' defaults with its `\setup` applied.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub values: Vec<(&'static str, String)>,
}

impl Style {
    pub fn new(kind: Kind) -> Self {
        Style { values: defaults(kind).iter().map(|(k, v)| (*k, v.to_string())).collect() }
    }

    /// Applies a `\setup{...}` option list; unknown keys are ignored, a flag means "true".
    pub fn update(&mut self, opts: &Options) {
        for (k, v) in &opts.items {
            if let Some(slot) = self.values.iter_mut().find(|(key, _)| key == k) {
                slot.1 = v.clone().unwrap_or_else(|| "true".to_string());
            }
        }
    }

    /// The value of a key; "" for a key this kind of diagram doesn't have.
    pub fn get(&self, key: &str) -> &str {
        self.values.iter().find(|(k, _)| *k == key).map_or("", |(_, v)| v.as_str())
    }

    /// The value of a key, or the error the Python app gave for a key this kind of diagram
    /// doesn't have (a `KeyError`).
    pub fn try_get(&self, key: &str) -> Result<&str, String> {
        self.values
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
            .ok_or_else(|| crate::text::py_repr(key))
    }

    pub fn length(&self, key: &str) -> f64 {
        parse_length_pt(self.get(key), 10.0)
    }

    pub fn number(&self, key: &str) -> f64 {
        parse_float(self.get(key)).unwrap_or(0.0)
    }

    pub fn flag(&self, key: &str) -> bool {
        crate::text::strip(self.get(key)) == "true"
    }

    /// A font key applied to `base`, or to the diagram's base font.
    pub fn font(&self, key: &str, base: Option<&Fmt>) -> Fmt {
        let default = Fmt { sans: self.flag("sans serif"), ..Fmt::default() };
        font_switches(self.get(key), base.unwrap_or(&default))
    }

    /// IS_A, CAN_BE, PART_OF in the language in use, unless the diagram has its own label.
    pub fn keyword(&self, key: &str, app_language: LabelLanguage) -> String {
        let value = self.get(key);
        let default = ER_DEFAULTS.iter().find(|(k, _)| *k == key).map_or("", |(_, v)| *v);
        if value != default {
            return value.to_string();
        }
        let language = match crate::text::strip(self.get("label language")) {
            "german" => LabelLanguage::German,
            "english" => LabelLanguage::English,
            _ => app_language,
        };
        match language {
            LabelLanguage::German => {
                GERMAN_LABELS.iter().find(|(k, _)| *k == key).map_or(value, |(_, v)| *v).to_string()
            }
            LabelLanguage::English => value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_and_keywords() {
        let mut st = Style::new(Kind::Er);
        st.update(&Options::parse(Some("sans serif, rel aspect=2, unknown=1")));
        assert!(st.flag("sans serif"));
        assert_eq!(st.number("rel aspect"), 2.0);
        assert!(st.font("entity font", None).sans);
        assert_eq!(st.keyword("isa label", LabelLanguage::German), "IST\\_EIN*E");
        st.update(&Options::parse(Some("label language=english")));
        assert_eq!(st.keyword("isa label", LabelLanguage::German), "IS\\_A");
        assert!((st.length("attr spread") - 0.35 * 28.45274).abs() < 1e-9);
    }
}
