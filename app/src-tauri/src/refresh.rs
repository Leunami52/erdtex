//! WebKit prefers to paint about 60 frames a second ("prefer page rendering updates near 60 fps"),
//! also on a 144 or 180 Hz monitor, where Qt (the Python app) follows the monitor.  The preference is
//! switched off here (WebKitGTK 2.42 and newer; the functions are looked up when the app starts, so
//! that it starts with older ones too).  Measured with WebKitGTK 2.54 on a 180 Hz monitor
//! (`e2e/measure_pan.py`), WebKitGTK still paints at 60 Hz: its own pacing, not this preference, so a
//! later WebKitGTK may make the difference.  `ERDTEX_DEBUG_REFRESH=1` says whether it was switched off.

#[cfg(target_os = "linux")]
pub fn follow_the_monitor(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|wv| {
        use std::ffi::{CStr, c_char, c_int, c_void};
        use webkit2gtk::WebViewExt;
        use webkit2gtk::glib::translate::ToGlibPtr;

        let debug = std::env::var_os("ERDTEX_DEBUG_REFRESH").is_some();
        let Some(settings) = wv.inner().settings() else { return };
        let ptr: *mut webkit2gtk::ffi::WebKitSettings = settings.to_glib_none().0;
        let find = |name: &CStr| {
            // SAFETY: looking a symbol up in the libraries already loaded; no pointer is kept
            let f = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
            (!f.is_null()).then_some(f)
        };
        let (Some(all), Some(len), Some(get), Some(ident), Some(set), Some(unref)) = (
            find(c"webkit_settings_get_all_features"),
            find(c"webkit_feature_list_get_length"),
            find(c"webkit_feature_list_get"),
            find(c"webkit_feature_get_identifier"),
            find(c"webkit_settings_set_feature_enabled"),
            find(c"webkit_feature_list_unref"),
        ) else {
            if debug {
                eprintln!("refresh: WebKitGTK has no feature list (older than 2.42)");
            }
            return; // WebKitGTK older than 2.42
        };
        // SAFETY: the signatures of WebKitGTK's documented C API (since 2.42), found above; the
        // list is released at the end, the features belong to it and aren't kept.
        unsafe {
            let all: unsafe extern "C" fn() -> *mut c_void = std::mem::transmute(all);
            let len: unsafe extern "C" fn(*mut c_void) -> usize = std::mem::transmute(len);
            let get: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void = std::mem::transmute(get);
            let ident: unsafe extern "C" fn(*mut c_void) -> *const c_char = std::mem::transmute(ident);
            let set: unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) = std::mem::transmute(set);
            let unref: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(unref);
            let list = all();
            if list.is_null() {
                return;
            }
            for k in 0..len(list) {
                let f = get(list, k);
                let id = ident(f);
                if !id.is_null() && CStr::from_ptr(id).to_bytes() == b"PreferPageRenderingUpdatesNear60FPS" {
                    set(ptr.cast(), f, 0);
                    if debug {
                        eprintln!("refresh: the 60 fps preference is switched off");
                    }
                }
            }
            unref(list);
        }
    });
}

#[cfg(not(target_os = "linux"))]
pub fn follow_the_monitor(_window: &tauri::WebviewWindow) {}
