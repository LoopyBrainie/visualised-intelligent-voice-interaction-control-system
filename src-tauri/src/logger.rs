// logger.rs - 日志系统模块
// 职责: 将日志通过 Tauri 事件发送到前端

use anyhow::Result;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

#[derive(Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: &str, message: &str) -> Self {
        let timestamp = chrono_lite_timestamp();
        Self {
            timestamp,
            level: level.to_string(),
            message: message.to_string(),
        }
    }
}

fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    let millis = duration.subsec_millis();
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

pub fn emit_log(level: &str, message: &str) {
    let entry = LogEntry::new(level, message);
    if let Ok(guard) = APP_HANDLE.lock() {
        if let Some(app) = guard.as_ref() {
            let _ = app.emit("log_event", entry);
        }
    }
}

pub fn init_logger(app: AppHandle) -> Result<()> {
    // 保存 AppHandle 以便后续发送日志事件
    if let Ok(mut guard) = APP_HANDLE.lock() {
        *guard = Some(app);
    }
    Ok(())
}

pub fn log_error(msg: &str) {
    emit_log("ERROR", msg);
    eprintln!("[ERROR] {}", msg);
}

pub fn log_warn(msg: &str) {
    emit_log("WARN", msg);
    eprintln!("[WARN] {}", msg);
}

pub fn log_info(msg: &str) {
    emit_log("INFO", msg);
    eprintln!("[INFO] {}", msg);
}

pub fn log_debug(msg: &str) {
    emit_log("DEBUG", msg);
    eprintln!("[DEBUG] {}", msg);
}
