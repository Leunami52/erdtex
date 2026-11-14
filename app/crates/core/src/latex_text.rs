//! Approximate typesetting of label markup for the canvas.
//!
//! Labels are LaTeX.  The canvas needs a faithful approximation: line breaks, bold, italic,
//! typewriter, underline, colour, font sizes, simple maths (italic letters, super- and
//! subscripts, a few symbols) and the usual special characters.  Unknown commands are
//! dropped, their arguments kept.

use crate::text::{is_alpha, is_space, round_half_even};

/// Font sizes relative to \normalsize = 10pt (article, 10pt).
pub const SIZES: &[(&str, f64)] = &[
    ("tiny", 5.0),
    ("scriptsize", 7.0),
    ("footnotesize", 8.0),
    ("small", 9.0),
    ("normalsize", 10.0),
    ("large", 12.0),
    ("Large", 14.4),
    ("LARGE", 17.28),
    ("huge", 20.74),
    ("Huge", 24.88),
];

/// Baseline skip per font size.
pub const BASELINE: &[(f64, f64)] = &[
    (5.0, 6.0),
    (7.0, 8.0),
    (8.0, 9.5),
    (9.0, 11.0),
    (10.0, 12.0),
    (12.0, 14.5),
    (14.4, 18.0),
    (17.28, 22.0),
    (20.74, 25.0),
    (24.88, 30.0),
];

const SYMBOLS: &[(&str, &str)] = &[
    ("textbar", "|"),
    ("textbackslash", "\\"),
    ("textasciitilde", "~"),
    ("textasciicircum", "^"),
    ("textless", "<"),
    ("textgreater", ">"),
    ("dots", "…"),
    ("ldots", "…"),
    ("textendash", "–"),
    ("textemdash", "—"),
    ("S", "§"),
    ("P", "¶"),
    ("copyright", "©"),
    ("textregistered", "®"),
    ("euro", "€"),
    ("textdegree", "°"),
    ("LaTeX", "LaTeX"),
    ("TeX", "TeX"),
    ("ss", "ß"),
    ("ae", "æ"),
    ("oe", "œ"),
    ("o", "ø"),
    ("aa", "å"),
    ("l", "ł"),
    ("quad", "\u{2003}"),
    ("qquad", "\u{2003}\u{2003}"),
    ("enspace", "\u{2002}"),
    ("thinspace", "\u{2009}"),
    ("space", " "),
];

const MATH_SYMBOLS: &[(&str, &str)] = &[
    ("neq", "≠"),
    ("ne", "≠"),
    ("leq", "≤"),
    ("le", "≤"),
    ("geq", "≥"),
    ("ge", "≥"),
    ("rightarrow", "→"),
    ("to", "→"),
    ("leftarrow", "←"),
    ("Rightarrow", "⇒"),
    ("Leftarrow", "⇐"),
    ("leftrightarrow", "↔"),
    ("Leftrightarrow", "⇔"),
    ("mapsto", "↦"),
    ("cdot", "·"),
    ("times", "×"),
    ("infty", "∞"),
    ("in", "∈"),
    ("notin", "∉"),
    ("subset", "⊂"),
    ("subseteq", "⊆"),
    ("supset", "⊃"),
    ("supseteq", "⊇"),
    ("cup", "∪"),
    ("cap", "∩"),
    ("emptyset", "∅"),
    ("varnothing", "∅"),
    ("forall", "∀"),
    ("exists", "∃"),
    ("neg", "¬"),
    ("wedge", "∧"),
    ("land", "∧"),
    ("vee", "∨"),
    ("lor", "∨"),
    ("pm", "±"),
    ("alpha", "α"),
    ("beta", "β"),
    ("gamma", "γ"),
    ("delta", "δ"),
    ("epsilon", "ε"),
    ("lambda", "λ"),
    ("mu", "μ"),
    ("pi", "π"),
    ("sigma", "σ"),
    ("tau", "τ"),
    ("phi", "φ"),
    ("omega", "ω"),
    ("Sigma", "Σ"),
    ("Pi", "Π"),
    ("Omega", "Ω"),
    ("Delta", "Δ"),
    ("ldots", "…"),
    ("cdots", "⋯"),
    ("mid", "|"),
    ("vert", "|"),
    ("langle", "⟨"),
    ("rangle", "⟩"),
    ("lbrace", "{"),
    ("rbrace", "}"),
    ("sum", "∑"),
    ("prod", "∏"),
    ("approx", "≈"),
    ("equiv", "≡"),
    ("sim", "∼"),
    ("circ", "∘"),
    ("bullet", "•"),
    ("star", "⋆"),
    ("ast", "∗"),
];

const ACCENTS: &[(&str, char)] = &[
    ("'", '\u{301}'),
    ("`", '\u{300}'),
    ("^", '\u{302}'),
    ("\"", '\u{308}'),
    ("~", '\u{303}'),
    ("c", '\u{327}'),
    ("=", '\u{304}'),
    ("v", '\u{30c}'),
];

fn lookup<'a, T: Copy>(table: &'a [(&'a str, T)], name: &str) -> Option<T> {
    table.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

/// How a label is underlined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Underline {
    #[default]
    None,
    Solid,
    /// a partial key
    Dashed,
}

/// The format of a run of text.
#[derive(Clone, Debug, PartialEq)]
pub struct Fmt {
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub sans: bool,
    pub underline: Underline,
    /// an xcolor expression
    pub color: Option<String>,
    /// pt
    pub size: f64,
    /// baseline shift in pt, for ^ and _
    pub rise: f64,
}

impl Default for Fmt {
    fn default() -> Self {
        Fmt {
            bold: false,
            italic: false,
            mono: false,
            sans: false,
            underline: Underline::None,
            color: None,
            size: 10.0,
            rise: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub text: String,
    pub fmt: Fmt,
}

/// Lines of formatted runs.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBlock {
    pub lines: Vec<Vec<Run>>,
    /// the length of each `\\[len]`
    pub extra_space: Vec<f64>,
}

impl TextBlock {
    pub fn plain(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().map(|r| r.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self) -> bool {
        !self.lines.iter().flatten().any(|r| !r.text.trim_matches(is_space).is_empty())
    }

    fn emit(&mut self, text: &str, fmt: &Fmt) {
        if text.is_empty() {
            return;
        }
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        if let Some(line) = self.lines.last_mut() {
            match line.last_mut() {
                Some(last) if last.fmt == *fmt => last.text.push_str(text),
                _ => line.push(Run { text: text.to_string(), fmt: fmt.clone() }),
            }
        }
    }

    fn newline(&mut self, extra: f64) {
        self.lines.push(Vec::new());
        self.extra_space.push(extra);
    }

    fn ends_with_space(&self) -> bool {
        self.lines.last().and_then(|l| l.last()).is_some_and(|r| r.text.ends_with(' '))
    }
}

/// `2pt` -> 2, `-0.1cm` -> -2.845, `1em` -> `em`; anything else -> 0.
pub fn parse_length_pt(text: &str, em: f64) -> f64 {
    // \s*([-+]?\d*\.?\d+)\s*(pt|cm|mm|in|em|ex|bp|sp)?\s*$
    let s = text.trim_start_matches(is_space);
    let b = s.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let int = b.iter().skip(i).take_while(|c| c.is_ascii_digit()).count();
    i += int;
    if b.get(i) == Some(&b'.') {
        let frac = b.iter().skip(i + 1).take_while(|c| c.is_ascii_digit()).count();
        if frac == 0 {
            return 0.0;
        }
        i += 1 + frac;
    } else if int == 0 {
        return 0.0;
    }
    let number = s.get(..i).unwrap_or("");
    let rest = s.get(i..).unwrap_or("").trim_start_matches(is_space);
    let (unit, rest) = ["pt", "cm", "mm", "in", "em", "ex", "bp", "sp"]
        .iter()
        .find_map(|u| rest.strip_prefix(u).map(|r| (*u, r)))
        .unwrap_or(("pt", rest));
    if !rest.chars().all(is_space) {
        return 0.0;
    }
    let Ok(v) = number.parse::<f64>() else { return 0.0 };
    v * match unit {
        "cm" => 28.45274,
        "mm" => 2.845274,
        "in" => 72.27,
        "em" => em,
        "ex" => em * 0.43,
        "bp" => 1.00375,
        "sp" => 1.0 / 65536.0,
        _ => 1.0,
    }
}

struct Parser<'b> {
    s: Vec<char>,
    i: usize,
    block: &'b mut TextBlock,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn starts_with(&self, pat: &str) -> bool {
        pat.chars().enumerate().all(|(k, c)| self.s.get(self.i + k) == Some(&c))
    }

    fn text(&self, a: usize, b: usize) -> String {
        self.s.get(a..b.min(self.s.len())).map_or(String::new(), |c| c.iter().collect())
    }

    fn sub(&mut self, src: &str, fmt: &Fmt, math: bool) -> Fmt {
        let mut p = Parser { s: src.chars().collect(), i: 0, block: &mut *self.block };
        p.parse(fmt.clone(), math)
    }

    /// Reads `{..}` (after optional spaces) and returns its content; a single token argument
    /// is returned as it is.
    fn read_group(&mut self) -> Option<String> {
        while self.peek() == Some(' ') {
            self.i += 1;
        }
        match self.peek() {
            Some('{') => {
                let mut depth = 0i64;
                let mut j = self.i;
                while let Some(&c) = self.s.get(j) {
                    if c == '\\' {
                        j += 2;
                        continue;
                    }
                    if c == '{' {
                        depth += 1;
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            let content = self.text(self.i + 1, j);
                            self.i = j + 1;
                            return Some(content);
                        }
                    }
                    j += 1;
                }
                let content = self.text(self.i + 1, self.s.len());
                self.i = self.s.len();
                Some(content)
            }
            Some('\\') => {
                let mut j = self.i + 1;
                while self.s.get(j).is_some_and(|&c| is_alpha(c)) {
                    j += 1;
                }
                let end = j.max(self.i + 2);
                let tok = self.text(self.i, end);
                self.i = end;
                Some(tok)
            }
            Some(c) => {
                self.i += 1;
                Some(c.to_string())
            }
            None => None,
        }
    }

    fn read_optional(&mut self) -> Option<String> {
        if self.peek() == Some('[')
            && let Some(k) = self.s.iter().skip(self.i).position(|&c| c == ']')
        {
            let j = self.i + k;
            let content = self.text(self.i + 1, j);
            self.i = j + 1;
            return Some(content);
        }
        None
    }

    fn read_command(&mut self) -> String {
        let j = self.i + 1;
        if self.s.get(j).is_some_and(|&c| is_alpha(c)) {
            let mut k = j;
            while self.s.get(k).is_some_and(|&c| is_alpha(c)) {
                k += 1;
            }
            let name = self.text(j, k);
            self.i = k;
            while self.peek() == Some(' ') {
                self.i += 1; // spaces after a control word vanish
            }
            name
        } else {
            let name = self.text(j, j + 1);
            self.i = j + 1;
            name
        }
    }

    /// Typesets to the end; returns the format active at the end (for switches like
    /// \bfseries).
    fn parse(&mut self, mut fmt: Fmt, math: bool) -> Fmt {
        while let Some(c) = self.peek() {
            match c {
                '\\' => fmt = self.command(fmt, math),
                '{' => {
                    let inner = self.read_group().unwrap_or_default();
                    self.sub(&inner, &fmt, math);
                }
                '}' => self.i += 1,
                '$' => {
                    self.i += 1;
                    let j = self
                        .s
                        .iter()
                        .skip(self.i)
                        .position(|&c| c == '$')
                        .map_or(self.s.len(), |k| self.i + k);
                    let inner = self.text(self.i, j);
                    self.i = j + 1;
                    self.sub(&inner, &fmt, true);
                }
                '^' | '_' if math => {
                    self.i += 1;
                    let arg = self.read_group().unwrap_or_default();
                    let shift = if c == '^' { 0.35 * fmt.size } else { -0.2 * fmt.size };
                    let sf = Fmt { size: fmt.size * 0.7, rise: fmt.rise + shift, ..fmt.clone() };
                    self.sub(&arg, &sf, true);
                }
                '~' => {
                    self.block.emit("\u{a0}", &fmt); // a non-breaking space
                    self.i += 1;
                }
                '%' => {
                    self.i = self
                        .s
                        .iter()
                        .skip(self.i)
                        .position(|&c| c == '\n')
                        .map_or(self.s.len(), |k| self.i + k + 1);
                }
                '-' if !math => {
                    if self.starts_with("---") {
                        self.block.emit("—", &fmt);
                        self.i += 3;
                    } else if self.starts_with("--") {
                        self.block.emit("–", &fmt);
                        self.i += 2;
                    } else {
                        self.block.emit("-", &fmt);
                        self.i += 1;
                    }
                }
                '`' | '\'' if !math && self.s.get(self.i + 1) == Some(&c) => {
                    self.block.emit(if c == '`' { "“" } else { "”" }, &fmt);
                    self.i += 2;
                }
                ' ' | '\t' | '\n' => {
                    if !self.block.ends_with_space() {
                        self.block.emit(" ", &fmt);
                    }
                    while matches!(self.peek(), Some(' ' | '\t' | '\n')) {
                        self.i += 1;
                    }
                }
                _ => {
                    if math && is_alpha(c) {
                        self.block.emit(&c.to_string(), &Fmt { italic: true, ..fmt.clone() });
                    } else if math && (c == '-' || c == '*') {
                        self.block
                            .emit(if c == '-' { "−" } else { "∗" }, &Fmt { italic: false, ..fmt.clone() });
                    } else {
                        self.block.emit(&c.to_string(), &fmt);
                    }
                    self.i += 1;
                }
            }
        }
        fmt
    }

    fn command(&mut self, fmt: Fmt, math: bool) -> Fmt {
        let name = self.read_command();
        let n = name.as_str();
        if n == "\\" {
            // line break, optional [len]
            if self.peek() == Some('*') {
                self.i += 1;
            }
            let extra = self
                .read_optional()
                .map_or(0.0, |o| if o.is_empty() { 0.0 } else { parse_length_pt(&o, fmt.size) });
            self.block.newline(extra);
            return fmt;
        }
        if n.is_empty() || (n.chars().count() == 1 && "_&%#${}".contains(n)) {
            self.block.emit(n, &fmt);
            return fmt;
        }
        if matches!(n, "," | ";" | ":" | ">") {
            self.block.emit("\u{2009}", &fmt); // a thin space
            return fmt;
        }
        if matches!(n, " " | "\n") {
            self.block.emit(" ", &fmt);
            return fmt;
        }
        if matches!(n, "!" | "/" | "-") {
            return fmt;
        }
        if let Some(accent) = lookup(ACCENTS, n) {
            let arg = self.read_group().unwrap_or_default();
            let mut chars = arg.chars();
            let mut out: String = chars.next().map(String::from).unwrap_or_default();
            out.push(accent);
            out.push_str(chars.as_str());
            self.block.emit(&out, &fmt);
            return fmt;
        }
        // font switches
        let switched = match n {
            "bfseries" => Some(Fmt { bold: true, ..fmt.clone() }),
            "mdseries" => Some(Fmt { bold: false, ..fmt.clone() }),
            "itshape" | "slshape" => Some(Fmt { italic: true, ..fmt.clone() }),
            "upshape" => Some(Fmt { italic: false, ..fmt.clone() }),
            "ttfamily" => Some(Fmt { mono: true, ..fmt.clone() }),
            "rmfamily" => Some(Fmt { mono: false, sans: false, ..fmt.clone() }),
            "sffamily" => Some(Fmt { mono: false, sans: true, ..fmt.clone() }),
            "normalfont" => Some(Fmt { bold: false, italic: false, mono: false, ..fmt.clone() }),
            _ => lookup(SIZES, n).map(|size| Fmt { size, ..fmt.clone() }),
        };
        if let Some(f) = switched {
            return f;
        }
        if n == "color" {
            self.read_optional();
            let col = self.read_group().unwrap_or_default();
            return Fmt { color: Some(col.trim_matches(is_space).to_string()), ..fmt };
        }
        // commands with an argument
        let wrapped = match n {
            "textbf" => Some(Fmt { bold: true, ..fmt.clone() }),
            "textit" | "textsl" | "mathit" => Some(Fmt { italic: true, ..fmt.clone() }),
            "emph" => Some(Fmt { italic: !fmt.italic, ..fmt.clone() }),
            "texttt" => Some(Fmt { mono: true, ..fmt.clone() }),
            "textrm" => Some(Fmt { mono: false, sans: false, ..fmt.clone() }),
            "textsf" => Some(Fmt { mono: false, sans: true, ..fmt.clone() }),
            "textup" | "text" | "mathrm" | "mathsf" | "operatorname" => {
                Some(Fmt { italic: false, ..fmt.clone() })
            }
            "textmd" => Some(Fmt { bold: false, ..fmt.clone() }),
            "textnormal" => Some(Fmt { bold: false, italic: false, ..fmt.clone() }),
            "underline" | "uline" => Some(Fmt { underline: Underline::Solid, ..fmt.clone() }),
            "dashuline" => Some(Fmt { underline: Underline::Dashed, ..fmt.clone() }),
            "mathbf" => Some(Fmt { bold: true, italic: false, ..fmt.clone() }),
            "mbox" | "hbox" | "textsc" | "makebox" | "fbox" => Some(fmt.clone()),
            _ => None,
        };
        if let Some(f2) = wrapped {
            if n == "makebox" {
                self.read_optional();
                self.read_optional();
            }
            let arg = self.read_group().unwrap_or_default();
            self.sub(&arg, &f2, math && !matches!(n, "text" | "mbox" | "hbox"));
            return fmt;
        }
        if n == "textcolor" {
            self.read_optional();
            let col = self.read_group().unwrap_or_default().trim_matches(is_space).to_string();
            let arg = self.read_group().unwrap_or_default();
            let f2 = Fmt { color: Some(col), ..fmt.clone() };
            self.sub(&arg, &f2, math);
            return fmt;
        }
        if matches!(
            n,
            "vspace"
                | "hspace"
                | "rule"
                | "phantom"
                | "hphantom"
                | "vphantom"
                | "label"
                | "index"
                | "footnote"
                | "hyperref"
                | "ref"
                | "cite"
        ) {
            if self.peek() == Some('*') {
                self.i += 1;
            }
            self.read_optional();
            let arg = self.read_group();
            if n == "hspace"
                && let Some(a) = arg.filter(|a| !a.is_empty())
            {
                let w = parse_length_pt(&a, fmt.size);
                if w > 0.0 {
                    let k = round_half_even(w / (fmt.size / 2.0)).max(1.0) as usize;
                    self.block.emit(&"\u{2002}".repeat(k), &fmt); // en spaces
                }
            }
            if n == "rule" {
                self.read_group();
            }
            return fmt;
        }
        if let Some(sym) = lookup(SYMBOLS, n) {
            self.block.emit(sym, &fmt);
            return fmt;
        }
        if let Some(sym) = lookup(MATH_SYMBOLS, n) {
            self.block.emit(sym, &Fmt { italic: false, ..fmt.clone() });
            return fmt;
        }
        if matches!(n, "left" | "right" | "big" | "Big" | "bigl" | "bigr") {
            return fmt;
        }
        if n == "frac" {
            let num = self.read_group().unwrap_or_default();
            let den = self.read_group().unwrap_or_default();
            self.sub(&num, &fmt, true);
            self.block.emit("/", &fmt);
            self.sub(&den, &fmt, true);
            return fmt;
        }
        // an unknown command: dropped, the {groups} that follow are typeset as text
        while self.peek() == Some('{') {
            let arg = self.read_group().unwrap_or_default();
            self.sub(&arg, &fmt, math);
        }
        fmt
    }
}

/// Label markup -> lines of formatted runs.
pub fn typeset(src: &str, base: &Fmt) -> TextBlock {
    let mut block = TextBlock { lines: vec![Vec::new()], extra_space: Vec::new() };
    let mut p = Parser { s: src.chars().collect(), i: 0, block: &mut block };
    p.parse(base.clone(), false);
    // spaces at the ends of lines go
    for line in &mut block.lines {
        if let Some(first) = line.first_mut() {
            first.text = first.text.trim_start_matches(' ').to_string();
        }
        if let Some(last) = line.last_mut() {
            last.text = last.text.trim_end_matches(' ').to_string();
        }
    }
    block
}

/// A label as plain text on one line (`main-\\tain` -> `maintain`).
pub fn plain_text(label: &str) -> String {
    if label.is_empty() {
        return String::new();
    }
    let s = typeset(label, &Fmt::default()).plain().replace("-\n", "");
    s.split(is_space).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Applies a font key's value such as `\small\bfseries` to a format.
pub fn font_switches(spec: &str, fmt: &Fmt) -> Fmt {
    let mut block = TextBlock { lines: vec![Vec::new()], extra_space: Vec::new() };
    let mut p = Parser { s: spec.chars().collect(), i: 0, block: &mut block };
    p.parse(fmt.clone(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_breaks_and_fonts() {
        let b = typeset("Student\\\\x", &Fmt::default());
        assert_eq!(b.plain(), "Student\nx");
        let b = typeset("a \\textbf{b} c", &Fmt::default());
        assert_eq!(b.lines[0].len(), 3);
        assert!(b.lines[0][1].fmt.bold);
        assert_eq!(plain_text("main-\\\\tain"), "maintain");
        assert_eq!(font_switches("\\small\\bfseries", &Fmt::default()).size, 9.0);
    }

    #[test]
    fn lengths() {
        assert_eq!(parse_length_pt("2pt", 10.0), 2.0);
        assert!((parse_length_pt(" -0.1cm ", 10.0) + 2.845274).abs() < 1e-9);
        assert_eq!(parse_length_pt("1.", 10.0), 0.0);
        assert_eq!(parse_length_pt("1em", 9.0), 9.0);
    }
}
