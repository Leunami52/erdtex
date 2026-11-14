//! Key/value options as written in `[...]`.

use crate::texscan::{split_top, strip_braces};
use crate::text::strip;

/// An ordered `key=value` list.  A value of `None` is a flag without value, such as `weak`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub items: Vec<(String, Option<String>)>,
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(text: Option<&str>) -> Self {
        let mut opts = Options::new();
        let Some(text) = text else { return opts };
        for part in split_top(text, ',') {
            if strip(&part).is_empty() {
                continue;
            }
            let kv = split_top(&part, '=');
            let key = strip(kv.first().map_or("", String::as_str)).to_string();
            if kv.len() == 1 {
                opts.items.push((key, None));
            } else {
                let value = kv.get(1..).map_or(String::new(), |v| v.join("="));
                opts.items.push((key, Some(strip_braces(strip(&value)).to_string())));
            }
        }
        opts
    }

    pub fn contains(&self, key: &str) -> bool {
        self.items.iter().any(|(k, _)| k == key)
    }

    /// A flag that is set: `key` or `key=true` (anything but `key=false`), as the packages read it.
    pub fn is_on(&self, key: &str) -> bool {
        self.contains(key) && self.get(key) != Some("false")
    }

    /// The value of `key`; `None` if it's missing *or* a flag (see `contains`).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.items.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.as_deref())
    }

    pub fn set(&mut self, key: &str, value: Option<&str>) {
        let value = value.map(str::to_string);
        match self.items.iter_mut().find(|(k, _)| k == key) {
            Some(item) => item.1 = value,
            None => self.items.push((key.to_string(), value)),
        }
    }

    pub fn remove(&mut self, key: &str) {
        self.items.retain(|(k, _)| k != key);
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|(k, _)| k.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `[a, b=c, d={x,y}]`, or "" if empty.
    pub fn to_tex(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = self
            .items
            .iter()
            .map(|(k, v)| match v {
                None => k.clone(),
                Some(v) if v.is_empty() || v.contains([',', '=', '[', ']']) || v.as_str() != strip(v) => {
                    format!("{k}={{{v}}}")
                }
                Some(v) => format!("{k}={v}"),
            })
            .collect();
        format!("[{}]", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_braces_and_parens() {
        let o = Options::parse(Some("via={(1,2) (3,4)}, bend=-20, role={a, b}, ->"));
        assert_eq!(o.get("via"), Some("(1,2) (3,4)"));
        assert_eq!(o.get("role"), Some("a, b"));
        assert!(o.contains("->") && o.get("->").is_none());
        assert_eq!(o.to_tex(), "[via={(1,2) (3,4)}, bend=-20, role={a, b}, ->]");
    }
}
