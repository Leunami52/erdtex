//! The desktop's own colours for "System" mode: a KDE Plasma colour scheme (`~/.config/kdeglobals`),
//! read as the Python app's `desktop.kde_palette` does.  Elsewhere the page's own light or dark
//! palette is used.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

type Rgb = (u8, u8, u8);

fn kdeglobals() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("kdeglobals"))
}

fn hex((r, g, b): Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Qt's `QColor::lighter(f)` / `darker(f)` (HSV value scaled, as Qt does it).
fn scale_value((r, g, b): Rgb, f: f64) -> Rgb {
    let (r, g, b) = (f64::from(r) / 255.0, f64::from(g) / 255.0, f64::from(b) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let (h, s, v) = {
        let d = max - min;
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / d).rem_euclid(6.0))
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        (h, if max == 0.0 { 0.0 } else { d / max }, max)
    };
    let mut v2 = v * f;
    let mut s2 = s;
    if v2 > 1.0 {
        // Qt: what doesn't fit into the value lowers the saturation
        s2 = (s - (v2 - 1.0)).max(0.0);
        v2 = 1.0;
    }
    let c = v2 * s2;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v2 - c;
    let (r1, g1, b1) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to(r1), to(g1), to(b1))
}

/// The page's palette (`window`, `text`, `base`, …) from a colour scheme's text.
pub fn palette_from(text: &str) -> Option<BTreeMap<String, String>> {
    let mut groups: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut cur: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            cur = Some(name.to_string());
            groups.entry(name.to_string()).or_default();
        } else if let (Some(g), Some((k, v))) = (&cur, line.split_once('=')) {
            groups.entry(g.clone()).or_default().insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    let col = |group: &str, key: &str| -> Option<Rgb> {
        let v = groups.get(&format!("Colors:{group}"))?.get(key)?;
        let p: Vec<u8> = v.split(',').take(3).filter_map(|x| x.trim().parse().ok()).collect();
        match p.as_slice() {
            [r, g, b] => Some((*r, *g, *b)),
            _ => None,
        }
    };
    let win_bg = col("Window", "BackgroundNormal")?;
    let win_fg = col("Window", "ForegroundNormal")?;
    let view_bg = col("View", "BackgroundNormal").unwrap_or(win_bg);
    let btn_bg = col("Button", "BackgroundNormal").unwrap_or(win_bg);
    let inactive = col("View", "ForegroundInactive").unwrap_or(win_fg);
    let colors: [(&str, Rgb); 12] = [
        ("window", win_bg),
        ("text", win_fg),
        ("base", view_bg),
        ("alternate", col("View", "BackgroundAlternate").unwrap_or(view_bg)),
        ("button", btn_bg),
        ("highlight", col("Selection", "BackgroundNormal").unwrap_or((61, 174, 233))),
        ("link", col("View", "ForegroundLink").unwrap_or((41, 128, 185))),
        ("placeholder", inactive),
        ("disabled", inactive),
        ("light", scale_value(btn_bg, 1.5)),
        ("mid", scale_value(btn_bg, 1.0 / 1.3)),
        ("dark", scale_value(btn_bg, 1.0 / 1.7)),
    ];
    Some(colors.iter().map(|(k, c)| (k.to_string(), hex(*c))).collect())
}

/// The desktop's palette, if this is KDE Plasma with a colour scheme.
pub fn palette() -> Option<BTreeMap<String, String>> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_uppercase();
    if !desktop.contains("KDE") {
        return None;
    }
    palette_from(&std::fs::read_to_string(kdeglobals()?).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breeze_dark() {
        let text = "[Colors:Window]\nBackgroundNormal=32,35,38\nForegroundNormal=252,252,252\n\
                    [Colors:View]\nBackgroundNormal=20,22,24\nBackgroundAlternate=29,31,34\n\
                    ForegroundInactive=161,169,177\nForegroundLink=29,153,243\n\
                    [Colors:Button]\nBackgroundNormal=41,44,48\n[Colors:Selection]\nBackgroundNormal=61,174,233\n";
        let p = palette_from(text).unwrap();
        assert_eq!(p["window"], "#202326");
        assert_eq!(p["base"], "#141618");
        assert_eq!(p["highlight"], "#3daee9");
        assert_eq!(p["mid"], "#202225"); // QColor(41,44,48).darker(130)
        assert!(palette_from("[General]\nx=1\n").is_none());
    }
}
