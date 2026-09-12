use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const APP_IDENTIFIER: &str = "com.tauri-app.goshootfilm";
static STARTUP_COMPLETE: AtomicBool = AtomicBool::new(false);

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        record("panic", &info.to_string());
        if !STARTUP_COMPLETE.load(Ordering::Relaxed) {
            show_startup_error(&log_path());
        }
        previous(info);
    }));
    record(
        "process",
        &format!("starting GoShootFilm {}", env!("CARGO_PKG_VERSION")),
    );
}

pub fn mark_startup_complete() {
    STARTUP_COMPLETE.store(true, Ordering::Relaxed);
}

pub fn record(stage: &str, detail: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let _ = writeln!(
        file,
        "[{}.{:03}] {stage}: {detail}",
        elapsed.as_secs(),
        elapsed.subsec_millis()
    );
}

pub fn report_error(stage: &str, error: &dyn Error) {
    let mut detail = format!("error[0]: {error}");
    let mut source = error.source();
    let mut depth = 1;
    while let Some(current) = source {
        detail.push_str(&format!("\nerror[{depth}]: {current}"));
        source = current.source();
        depth += 1;
    }
    record(stage, &detail);
    show_startup_error(&log_path());
}

pub fn log_path() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_IDENTIFIER)
        .join("logs")
        .join("startup.log")
}

#[cfg(windows)]
fn show_startup_error(path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let message = format!(
        "GoShootFilm 启动失败。\n\n详细信息已写入：\n{}",
        path.display()
    );
    let message: Vec<u16> = std::ffi::OsStr::new(&message)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let title: Vec<u16> = std::ffi::OsStr::new("GoShootFilm 启动错误")
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_startup_error(_path: &std::path::Path) {}
