//! The Rust side of the Tauri app: what a web page can't do.  Files, settings, the paths given
//! on the command line, pdflatex and the exports.  The document itself lives in the page, in the core compiled to WebAssembly.

mod desktop;
mod export;
mod files;
mod latex;
mod refresh;
mod selftest;
mod settings;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use tauri::Manager;
use tauri::ipc::Response;

use settings::Settings;

static START: OnceLock<Instant> = OnceLock::new();

/// Reads a diagram file: its absolute path, text (with `\n` line ends) and kind.
#[tauri::command]
fn read_diagram(path: String) -> Result<files::Opened, String> {
    files::read(Path::new(&path))
}

/// Writes a diagram file; returns its absolute path.
#[tauri::command]
fn write_diagram(path: String, text: String) -> Result<String, String> {
    files::write(Path::new(&path), &text)
}

/// Writes a text file (a relational schema, SQL); returns its absolute path.
#[tauri::command]
fn write_text(path: String, text: String) -> Result<String, String> {
    std::fs::write(&path, text).map_err(|e| format!("Could not save:\n{e}"))?;
    Ok(files::absolute(Path::new(&path)).display().to_string())
}

/// The kind of diagram an existing file holds, if any.
#[tauri::command]
fn file_kind(path: String) -> Option<&'static str> {
    files::kind_of(Path::new(&path))
}

/// The absolute form of a path.
#[tauri::command]
fn absolute_path(path: String) -> String {
    files::absolute(Path::new(&path)).display().to_string()
}

/// Whether a file exists.
#[tauri::command]
fn file_exists(path: String) -> bool {
    Path::new(&path).is_file()
}

/// Whether copying stays inside the app (tests set ERDTEX_PRIVATE_CLIPBOARD, so that the user's
/// clipboard stays as it is).
#[tauri::command]
fn private_clipboard() -> bool {
    std::env::var_os("ERDTEX_PRIVATE_CLIPBOARD").is_some()
}

/// The files named on the command line.
#[tauri::command]
fn startup_paths() -> Vec<String> {
    std::env::args().skip(1).filter(|a| !a.starts_with('-')).collect()
}

#[tauri::command]
fn get_settings(settings: tauri::State<'_, Settings>) -> BTreeMap<String, String> {
    settings.all()
}

#[tauri::command]
fn set_setting(
    settings: tauri::State<'_, Settings>,
    key: String,
    value: Option<String>,
) -> Result<(), String> {
    settings.set(&key, value)
}

fn kind(name: &str) -> diagram_core::document::Kind {
    diagram_core::document::Kind::from_name(name).unwrap_or(diagram_core::document::Kind::Er)
}

/// Runs blocking work (pdflatex, exports) away from the window's thread.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

/// Empty if pdflatex is there, else what to install.
#[tauri::command]
fn latex_missing() -> String {
    if latex::which("pdflatex").is_some() {
        String::new()
    } else {
        format!("pdflatex was not found — the exact preview needs {}.", latex::TEX_HINT)
    }
}

/// The diagram compiled by pdflatex: the PDF, or its errors (one per line).
#[tauri::command]
async fn latex_compile(
    text: String,
    kind: String,
    preamble: String,
    language: String,
    grid: bool,
) -> Result<Response, String> {
    blocking(move || {
        let job = latex::Job {
            text: &text,
            kind: self::kind(&kind),
            preamble: &preamble,
            language: &language,
            grid,
        };
        latex::compile(&job).map(Response::new).map_err(|e| e.join("\n"))
    })
    .await?
}

/// Exports the editor rendering: SVG from the core, PDF through svg2pdf, PNG through resvg.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn export_editor(
    text: String,
    kind: String,
    language: String,
    format: String,
    grid: bool,
    dpi: u32,
    transparent: bool,
    path: String,
) -> Result<(), String> {
    blocking(move || {
        let st = export::Settings { format, grid, dpi, transparent };
        export::editor(&text, self::kind(&kind), &language, &st, Path::new(&path))
    })
    .await?
}

/// Exports the LaTeX result.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn export_latex(
    text: String,
    kind: String,
    language: String,
    preamble: String,
    format: String,
    grid: bool,
    dpi: u32,
    transparent: bool,
    path: String,
) -> Result<(), String> {
    blocking(move || {
        let st = export::Settings { format, grid, dpi, transparent };
        let job = latex::Job {
            text: &text,
            kind: self::kind(&kind),
            preamble: &preamble,
            language: &language,
            grid,
        };
        export::latex(&job, &st, Path::new(&path)).map_err(|e| e.join("\n"))
    })
    .await?
}

/// Writes bytes the page made (the LaTeX result as PNG where pdftocairo is missing): the body
/// is the bytes, the header `path` says where.
#[tauri::command]
fn write_bytes(request: tauri::ipc::Request<'_>) -> Result<(), String> {
    let path =
        request.headers().get("path").and_then(|v| v.to_str().ok()).map(percent_decode).ok_or("no path")?;
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err("no bytes".into()) };
    std::fs::write(&path, bytes).map_err(|e| format!("cannot write {path}: {e}"))
}

/// `%xx` escapes (a header carries only ASCII; the page encodes the path).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        let hex = b
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (c, hex) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// When a file last changed and its size (to notice changes by other programs).
#[tauri::command]
fn file_stamp(path: String) -> Option<(f64, u64)> {
    let m = std::fs::metadata(path).ok()?;
    let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some((t.as_secs_f64() * 1000.0, m.len()))
}

/// The paths of a list that are diagrams (for Open Recent; missing ones with a diagram's
/// extension stay, as in the Python app).
#[tauri::command]
fn diagram_paths(paths: Vec<String>) -> Vec<String> {
    paths.into_iter().filter(|p| files::is_diagram_file(Path::new(p))).collect()
}

/// Milliseconds since the Rust side started (for the page's own timestamps).
#[tauri::command]
fn since_start() -> f64 {
    START.get().map_or(0.0, |s| s.elapsed().as_secs_f64() * 1000.0)
}

/// The page calls this once the first diagram is on screen: milliseconds since start.  With
/// `ERDTEX_MEASURE_STARTUP` set, it prints them and ends the app (Milestone 0's measurements).
#[tauri::command]
fn ready(app: tauri::AppHandle, marks: String) -> f64 {
    let ms = since_start();
    if std::env::var_os("ERDTEX_MEASURE_STARTUP").is_some() {
        println!("ready_ms={ms:.1} page: {marks}");
        app.exit(0);
    }
    ms
}

/// The desktop's colours for "System" mode (a KDE colour scheme), if there are any.
#[tauri::command]
fn desktop_palette() -> Option<BTreeMap<String, String>> {
    desktop::palette()
}

/// The page asks whether it is running the self-test (`--self-test`).
#[tauri::command]
fn self_test_mode() -> bool {
    std::env::var_os("ERDTEX_SELF_TEST").is_some()
}

/// A line of the page's part of the self-test, printed at once (if the page stops, the last
/// line says where).
#[tauri::command]
fn self_test_line(line: String) {
    println!("{line}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// The end of the page's part of the self-test; the app ends with the result.
#[tauri::command]
fn self_test_report(app: tauri::AppHandle, ok: bool) {
    finish(ok && SELF_TEST_OK.get().copied().unwrap_or(false));
    app.exit(if ok { 0 } else { 1 });
}

static SELF_TEST_OK: OnceLock<bool> = OnceLock::new();

fn finish(ok: bool) {
    println!("{}", if ok { "all checks passed" } else { "SOME CHECKS FAILED" });
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// A program for the Windows desktop has no console of its own: the self-test's report goes to
/// the console it was started from.
fn attach_console() {
    #[cfg(windows)]
    {
        // SAFETY: a plain Win32 call without pointers; failing (no parent console) is harmless.
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(
                windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
    }
}

/// `--self-test [--no-window]`: the checks without a window; then the window's, in a settings
/// folder of its own.  `false`: the app doesn't start.
fn self_test(args: &[String]) -> bool {
    attach_console();
    let r = selftest::headless(env!("CARGO_PKG_VERSION"));
    for l in &r.lines {
        println!("{l}");
    }
    let _ = SELF_TEST_OK.set(r.ok);
    if !r.ok || args.iter().any(|a| a == "--no-window") {
        finish(r.ok);
        std::process::exit(if r.ok { 0 } else { 1 });
    }
    let dir = std::env::temp_dir().join(format!("erdtex-selftest-{}", std::process::id()));
    // SAFETY: still the only thread (the window hasn't started), so nothing reads the
    // environment meanwhile.
    unsafe {
        std::env::set_var("ERDTEX_CONFIG_DIR", &dir);
        std::env::set_var("ERDTEX_SELF_TEST", "1");
        std::env::set_var("ERDTEX_PRIVATE_CLIPBOARD", "1");
    }
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(90));
        println!(
            "FAIL  the window: the page didn't finish within 90 s (the last line above is the last step it reached)"
        );
        finish(false);
        std::process::exit(1);
    });
    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    START.get_or_init(Instant::now);
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--self-test") {
        self_test(&args);
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // ERDTEX_CONFIG_DIR: another folder for the settings (tests use one of their own)
            let dir = std::env::var_os("ERDTEX_CONFIG_DIR")
                .map(std::path::PathBuf::from)
                .or_else(|| app.path().app_config_dir().ok());
            let settings = Settings::load(dir.map(|d| d.join("settings.toml")));
            app.manage(settings);
            if let Some(w) = app.get_webview_window("main") {
                refresh::follow_the_monitor(&w);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_diagram,
            write_diagram,
            write_text,
            file_kind,
            absolute_path,
            file_exists,
            startup_paths,
            private_clipboard,
            get_settings,
            set_setting,
            latex_missing,
            latex_compile,
            export_latex,
            export_editor,
            write_bytes,
            file_stamp,
            diagram_paths,
            self_test_mode,
            desktop_palette,
            self_test_report,
            self_test_line,
            since_start,
            ready
        ])
        .run(tauri::generate_context!())
        .expect("error while running ERDTeX");
}
