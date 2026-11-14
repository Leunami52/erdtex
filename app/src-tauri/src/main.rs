// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK asks NetworkManager whether the network is up, and NetworkManager can take
    // seconds to answer, which delays the window by as much (Milestone 0). The app doesn't
    // need the network: GIO's simple monitor answers at once.
    #[cfg(target_os = "linux")]
    if std::env::var_os("GIO_USE_NETWORK_MONITOR").is_none() {
        // SAFETY: the first statement of main, before any other thread exists, so nothing
        // can read the environment at the same time.
        unsafe { std::env::set_var("GIO_USE_NETWORK_MONITOR", "base") };
    }
    erdtex_lib::run()
}
