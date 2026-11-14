//! The app's settings: a flat table of strings in `settings.toml` in the usual config folder
//! (`~/.config/<app>/` on Linux, `%APPDATA%\<app>\` on Windows).  Lists are kept as JSON in a string.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// The settings and the file they live in.
pub struct Settings {
    file: Option<PathBuf>,
    values: Mutex<BTreeMap<String, String>>,
}

impl Settings {
    /// Reads the file; a missing or broken file gives empty settings.
    pub fn load(file: Option<PathBuf>) -> Settings {
        let values = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|t| toml::from_str::<BTreeMap<String, String>>(&t).ok())
            .unwrap_or_default();
        Settings { file, values: Mutex::new(values) }
    }

    /// Everything, for the page.
    pub fn all(&self) -> BTreeMap<String, String> {
        self.values.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Sets (or with `None` removes) a value and writes the file.
    pub fn set(&self, key: &str, value: Option<String>) -> Result<(), String> {
        let mut values = self.values.lock().map_err(|e| e.to_string())?;
        match value {
            Some(v) => values.insert(key.to_string(), v),
            None => values.remove(key),
        };
        let Some(file) = &self.file else { return Ok(()) };
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = toml::to_string(&*values).map_err(|e| e.to_string())?;
        std::fs::write(file, text).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_in_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("sub/settings.toml");
        let s = Settings::load(Some(f.clone()));
        s.set("display mode", Some("dark".into())).unwrap();
        s.set("open tabs", Some("[\"/a b.erd\"]".into())).unwrap();
        let again = Settings::load(Some(f));
        assert_eq!(again.all().get("display mode").map(String::as_str), Some("dark"));
        again.set("display mode", None).unwrap();
        assert!(!again.all().contains_key("display mode"));
    }
}
