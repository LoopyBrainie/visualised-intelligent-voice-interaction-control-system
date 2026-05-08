// logger.rs - 日志系统模块
// 职责: 将日志通过 Tauri 事件发送到前端，支持 trace_id 全链路追踪

use anyhow::Result;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

/// 日志条目 - 支持 trace_id 全链路追踪
#[derive(Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl LogEntry {
    pub fn new(level: &str, message: &str) -> Self {
        let timestamp = chrono_lite_timestamp();
        Self {
            timestamp,
            level: level.to_string(),
            message: message.to_string(),
            trace_id: None,
            source: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: &str) -> Self {
        self.trace_id = Some(trace_id.to_string());
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
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

/// 生成新的 trace_id
pub fn new_trace_id() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

fn emit_log(level: &str, message: &str, trace_id: Option<&str>, source: Option<&str>) {
    let mut entry = LogEntry::new(level, message);
    if let Some(tid) = trace_id {
        entry = entry.with_trace_id(tid);
    }
    if let Some(s) = source {
        entry = entry.with_source(s);
    }
    if let Ok(guard) = APP_HANDLE.lock() {
        if let Some(app) = guard.as_ref() {
            let _ = app.emit("log_event", entry);
        }
    }
    // 同时打印到 stderr 用于本地调试
    let tid_str = trace_id.map(|t| format!(" [{}]", t)).unwrap_or_default();
    let src_str = source.map(|s| format!("<{}>", s)).unwrap_or_default();
    eprintln!("[{}]{} {}: {}{}", level, src_str, chrono_lite_timestamp(), message, tid_str);
}

pub fn init_logger(app: AppHandle) -> Result<()> {
    if let Ok(mut guard) = APP_HANDLE.lock() {
        *guard = Some(app);
    }
    Ok(())
}

pub fn log_error(msg: &str) {
    emit_log("ERROR", msg, None, None);
}

pub fn log_error_t(msg: &str, trace_id: &str) {
    emit_log("ERROR", msg, Some(trace_id), None);
}

pub fn log_error_s(msg: &str, source: &str) {
    emit_log("ERROR", msg, None, Some(source));
}

pub fn log_error_ts(msg: &str, trace_id: &str, source: &str) {
    emit_log("ERROR", msg, Some(trace_id), Some(source));
}

pub fn log_warn(msg: &str) {
    emit_log("WARN", msg, None, None);
}

pub fn log_warn_t(msg: &str, trace_id: &str) {
    emit_log("WARN", msg, Some(trace_id), None);
}

pub fn log_warn_s(msg: &str, source: &str) {
    emit_log("WARN", msg, None, Some(source));
}

pub fn log_warn_ts(msg: &str, trace_id: &str, source: &str) {
    emit_log("WARN", msg, Some(trace_id), Some(source));
}

pub fn log_info(msg: &str) {
    emit_log("INFO", msg, None, None);
}

pub fn log_info_t(msg: &str, trace_id: &str) {
    emit_log("INFO", msg, Some(trace_id), None);
}

pub fn log_info_s(msg: &str, source: &str) {
    emit_log("INFO", msg, None, Some(source));
}

pub fn log_info_ts(msg: &str, trace_id: &str, source: &str) {
    emit_log("INFO", msg, Some(trace_id), Some(source));
}

pub fn log_debug(msg: &str) {
    emit_log("DEBUG", msg, None, None);
}

pub fn log_debug_t(msg: &str, trace_id: &str) {
    emit_log("DEBUG", msg, Some(trace_id), None);
}

pub fn log_debug_s(msg: &str, source: &str) {
    emit_log("DEBUG", msg, None, Some(source));
}

pub fn log_debug_ts(msg: &str, trace_id: &str, source: &str) {
    emit_log("DEBUG", msg, Some(trace_id), Some(source));
}

/// 日志代理 - 将 Python daemon 回传的日志发送到前端
pub fn log_proxy(log_entry: &LogEntry) {
    if let Ok(guard) = APP_HANDLE.lock() {
        if let Some(app) = guard.as_ref() {
            let _ = app.emit("log_event", log_entry.clone());
        }
    }
}
