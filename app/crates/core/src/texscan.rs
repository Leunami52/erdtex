//! Low-level scanner for the diagram file syntax.
//!
//! The files are LaTeX, but only a tiny, regular subset matters: a sequence of commands
//! `\name[opts]{arg}(x,y){arg}` with comments and whitespace in between.  The scanner mimics
//! how TeX/xparse look for (optional) arguments, so that what we parse is what LaTeX sees:
//!
//! * spaces and a single line break are skipped before arguments,
//! * a `%` comment hides the rest of the line (including the line break),
//! * an empty line (TeX's `\par`) ends the search for optional arguments.
//!
//! Positions are byte offsets; every delimiter is ASCII, so they are always char boundaries.

use std::fmt;

use crate::text::{from, is_alpha, slice, strip};

/// A syntax error with its position, as "line L, column C: message".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    pub fn new(message: impl Into<String>, text: &str, pos: usize) -> Self {
        let before = slice(text, 0, pos);
        let line = before.matches('\n').count() + 1;
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        let column = from(before, line_start).chars().count() + 1;
        ParseError { message: message.into(), line, column }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// The kinds of arguments, as xparse names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arg {
    /// `O`: optional `[...]`
    Options,
    /// `m`: mandatory `{...}`
    Mandatory,
    /// `d`: optional `(...)`
    OptionalCoord,
    /// `r`: mandatory `(...)`
    Coord,
    /// `g`: optional trailing `{...}`
    OptionalGroup,
}

impl Arg {
    pub fn from_letter(c: char) -> Option<Arg> {
        Some(match c {
            'O' => Arg::Options,
            'm' => Arg::Mandatory,
            'd' => Arg::OptionalCoord,
            'r' => Arg::Coord,
            'g' => Arg::OptionalGroup,
            _ => return None,
        })
    }

    fn delimiters(self) -> (u8, u8, bool) {
        match self {
            Arg::Options => (b'[', b']', false),
            Arg::Mandatory => (b'{', b'}', true),
            Arg::OptionalCoord => (b'(', b')', false),
            Arg::Coord => (b'(', b')', true),
            Arg::OptionalGroup => (b'{', b'}', false),
        }
    }
}

pub struct Scanner<'a> {
    pub text: &'a str,
    bytes: &'a [u8],
    pub pos: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(text: &'a str) -> Self {
        Scanner { text, bytes: text.as_bytes(), pos: 0 }
    }

    pub fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    pub fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn at(&self, i: usize) -> Option<u8> {
        self.bytes.get(i).copied()
    }

    pub fn error(&self, message: impl Into<String>, pos: usize) -> ParseError {
        ParseError::new(message, self.text, pos)
    }

    /// `i` points at '%': the index after the end of the line.
    fn skip_comment(&self, i: usize) -> usize {
        self.bytes.iter().skip(i).position(|&b| b == b'\n').map_or(self.bytes.len(), |k| i + k + 1)
    }

    /// Skips all white space and comments (between statements); returns what was skipped.
    pub fn skip_trivia(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' => self.pos += 1,
                b'%' => self.pos = self.skip_comment(self.pos),
                _ => break,
            }
        }
        slice(self.text, start, self.pos)
    }

    /// Spaces, an optional comment and the line break after a statement, if nothing else
    /// follows on the same line; "" otherwise.
    pub fn read_line_rest(&mut self) -> &'a str {
        let mut i = self.pos;
        while matches!(self.at(i), Some(b' ' | b'\t' | b'\r')) {
            i += 1;
        }
        match self.at(i) {
            Some(b'%') => i = self.skip_comment(i),
            Some(b'\n') => i += 1,
            Some(_) => return "",
            None => {}
        }
        let rest = slice(self.text, self.pos, i);
        self.pos = i;
        rest
    }

    /// Where `opener` is, if TeX would find it as the next argument (skipping spaces, one line
    /// break and comments, but not a blank line).  Doesn't move the scanner.
    pub fn lookahead(&self, opener: u8) -> Option<usize> {
        let mut i = self.pos;
        let mut new_line = false; // TeX state N: at the beginning of a line
        while let Some(c) = self.at(i) {
            match c {
                b' ' | b'\t' | b'\r' => i += 1,
                b'\n' => {
                    if new_line {
                        return None; // a blank line is \par
                    }
                    new_line = true;
                    i += 1;
                }
                b'%' => {
                    i = self.skip_comment(i);
                    new_line = true;
                }
                c if c == opener => return Some(i),
                _ => return None,
            }
        }
        None
    }

    /// `i` points at `opener`: (inner text, index after the closer).  Escaped characters and
    /// comments are respected; braces are always balanced, the opener/closer pair is nested.
    pub fn read_balanced(&self, i: usize, opener: u8, closer: u8) -> Result<(&'a str, usize), ParseError> {
        let mut depth = 0i64;
        let mut braces = 0i64;
        let mut j = i;
        while let Some(c) = self.at(j) {
            if c == b'\\' {
                j += 2;
                continue;
            }
            if c == b'%' {
                j = self.skip_comment(j);
                continue;
            }
            if opener != b'{' {
                if c == b'{' {
                    braces += 1;
                } else if c == b'}' {
                    braces -= 1;
                    if braces < 0 {
                        return Err(self.error(
                            format!("unbalanced '}}' inside '{}...{}'", opener as char, closer as char),
                            j,
                        ));
                    }
                }
            }
            if braces == 0 {
                if c == opener {
                    depth += 1;
                } else if c == closer {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((slice(self.text, i + 1, j), j + 1));
                    }
                }
            }
            j += 1;
        }
        Err(self.error(format!("missing '{}'", closer as char), i))
    }

    /// At a backslash: reads a control word (letters) or a control symbol.
    pub fn read_name(&mut self) -> &'a str {
        let rest = from(self.text, self.pos + 1);
        let mut chars = rest.char_indices();
        let end = match chars.next() {
            Some((_, c)) if is_alpha(c) => {
                rest.char_indices().find(|&(_, c)| !is_alpha(c)).map_or(rest.len(), |(k, _)| k)
            }
            Some((_, c)) => c.len_utf8(),
            None => 0,
        };
        let name = slice(rest, 0, end);
        self.pos = (self.pos + 1 + end).min(self.bytes.len());
        name
    }

    /// Reads one argument; `None` for an optional argument that isn't there.
    pub fn read_arg(&mut self, kind: Arg) -> Result<Option<&'a str>, ParseError> {
        let (opener, closer, required) = kind.delimiters();
        match self.lookahead(opener) {
            Some(i) => {
                let (inner, end) = self.read_balanced(i, opener, closer)?;
                self.pos = end;
                Ok(Some(inner))
            }
            None if required => Err(self.error(format!("expected '{}'", opener as char), self.pos)),
            None => Ok(None),
        }
    }

    /// Skips to after the next ';' that isn't inside a group (raw TikZ).
    pub fn read_until_semicolon(&mut self) -> Result<(), ParseError> {
        let mut j = self.pos;
        let mut stack: Vec<u8> = Vec::new();
        while let Some(c) = self.at(j) {
            match c {
                b'\\' => {
                    j += 2;
                    continue;
                }
                b'%' => {
                    j = self.skip_comment(j);
                    continue;
                }
                b'{' => stack.push(b'}'),
                b'[' => stack.push(b']'),
                b'(' => stack.push(b')'),
                c if stack.last() == Some(&c) => {
                    stack.pop();
                }
                b';' if stack.is_empty() => {
                    self.pos = j + 1;
                    return Ok(());
                }
                _ => {}
            }
            j += 1;
        }
        Err(self.error("missing ';' after TikZ command", self.pos))
    }
}

/// Splits at separators that aren't inside {} [] () (escapes respected).
pub fn split_top(s: &str, sep: char) -> Vec<String> {
    split(s, sep, |c| matches!(c, '{' | '[' | '('), |c| matches!(c, '}' | ']' | ')'))
}

/// Splits like an expl3 clist/seq split: only braces protect a separator (parentheses don't;
/// `\,` is a command, not a separator).
pub fn split_tex(s: &str, sep: char) -> Vec<String> {
    split(s, sep, |c| c == '{', |c| c == '}')
}

fn split(s: &str, sep: char, open: impl Fn(char) -> bool, close: impl Fn(char) -> bool) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i64;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&next) = chars.peek()
        {
            cur.push(c);
            cur.push(next);
            chars.next();
            continue;
        }
        if open(c) {
            depth += 1;
        } else if close(c) {
            depth -= 1;
        }
        if c == sep && depth == 0 {
            parts.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    parts.push(cur);
    parts
}

/// `{abc}` -> `abc` if the braces enclose the whole (stripped) string.
pub fn strip_braces(s: &str) -> &str {
    let s = strip(s);
    let b = s.as_bytes();
    if b.len() >= 2 && b.first() == Some(&b'{') && b.last() == Some(&b'}') {
        let mut depth = 0i64;
        for (k, &c) in b.iter().enumerate() {
            if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
                if depth == 0 && k != b.len() - 1 {
                    return s;
                }
            }
        }
        return slice(s, 1, s.len() - 1);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_across_a_line_break_but_not_a_blank_line() {
        let mut sc = Scanner::new("  % c\n {x}");
        assert_eq!(sc.read_arg(Arg::OptionalGroup), Ok(Some("x")));
        let mut sc = Scanner::new("\n\n{x}");
        assert_eq!(sc.read_arg(Arg::OptionalGroup), Ok(None));
    }

    #[test]
    fn errors_have_positions() {
        let mut sc = Scanner::new("a\n (1,2");
        sc.pos = 1;
        let e = sc.read_arg(Arg::Coord).err();
        assert_eq!(e.map(|e| e.to_string()), Some("line 2, column 2: missing ')'".into()));
    }

    #[test]
    fn splitting() {
        assert_eq!(split_top("a{b,c},(d,e)", ','), ["a{b,c}", "(d,e)"]);
        assert_eq!(split_tex("(1,2)", ','), ["(1", "2)"]);
        assert_eq!(split_tex("a\\,b", ','), ["a\\,b"]);
        assert_eq!(strip_braces(" {a}{b} "), "{a}{b}");
        assert_eq!(strip_braces("{{x}}"), "{x}");
    }
}
