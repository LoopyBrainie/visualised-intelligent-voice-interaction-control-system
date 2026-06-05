// error.rs - 统一代数式错误类型
// 遵循 /algebraic-error-handling 三法则和 /rust-errors discriminated union 模式
//
// 核心设计:
// - #[error("...")] 仅用于 Rust 侧 Display/日志，不影响 serde 序列化
// - 智能构造函数在构造时格式化中文消息写入 message 字段
// - #[serde(tag = "name")] 内部标记 → TypeScript discriminated union
// - 不实现 From<String> 和 From<anyhow::Error>，强制显式选择 variant

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 统一应用错误类型 —— 三层共享的代数式错误
///
/// 序列化为: {"name": "VlmTimeout", "message": "VLM 请求超时"}
/// TypeScript 侧通过 arktype discriminated union 校验和 switch 分发
#[must_use]
#[derive(Error, Debug, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum AppError {
    /// 语音指令无法识别
    #[error("无法识别的指令: {input}")]
    UnrecognizedCommand {
        input: String,
        message: String,
    },

    /// VLM HTTP 请求超时
    #[error("VLM 请求超时")]
    VlmTimeout {
        #[serde(default)]
        detail: Option<String>,
        message: String,
    },

    /// VLM 返回的 JSON 无法解析
    #[error("VLM 响应解析失败: {detail}")]
    VlmParseError {
        detail: String,
        #[serde(skip)]
        #[source]
        cause: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },

    /// VLM 识别置信度过低
    #[error("VLM 置信度低: {confidence:.2} < 0.6")]
    VlmLowConfidence {
        confidence: f64,
        message: String,
    },

    /// 指令参数数值超出有效范围
    #[error("无效数值: 期望 {expected}，实际 {got}")]
    InvalidValue {
        expected: String,
        got: u8,
        message: String,
    },

    /// 目标设备在系统中不存在
    #[error("设备未找到: {device}")]
    DeviceNotFound {
        device: String,
        message: String,
    },

    /// Mutex/RwLock 中毒
    #[error("设备锁定失败: {lock_name}")]
    LockError {
        lock_name: String,
        message: String,
    },

    /// cpal 音频采集设备错误
    #[error("音频设备错误: {detail}")]
    AudioDevice {
        detail: String,
        message: String,
    },

    /// nokhwa 摄像头错误
    #[error("摄像头错误: {detail}")]
    CameraError {
        detail: String,
        message: String,
    },

    /// HTTP 网络请求错误
    #[error("网络错误: {detail}")]
    NetworkError {
        detail: String,
        message: String,
    },

    /// PyO3 / Python 环境错误
    #[error("Python 环境错误: {detail}")]
    PythonEnv {
        detail: String,
        message: String,
    },

    /// Python Flask Daemon 错误
    #[error("Daemon 错误: {detail}")]
    DaemonError {
        detail: String,
        message: String,
    },

    /// MediaPipe 手势识别错误
    #[error("手势识别错误: {detail}")]
    GestureError {
        detail: String,
        message: String,
    },

    /// 未分类的内部错误（兜底 variant）
    #[error("内部错误: {detail}")]
    InternalError {
        detail: String,
        #[serde(skip)]
        #[source]
        cause: Option<Box<dyn std::error::Error + Send + Sync>>,
        message: String,
    },
}

// ============================================================
// 智能构造函数 —— 负责格式化 message 字段
// ============================================================

impl AppError {
    pub fn unrecognized(input: impl Into<String>) -> Self {
        let input = input.into();
        let message = format!("无法识别的指令: {input}");
        Self::UnrecognizedCommand { input, message }
    }

    pub fn vlm_timeout() -> Self {
        Self::VlmTimeout {
            detail: None,
            message: "VLM 请求超时，请检查 AI 服务是否正常运行".into(),
        }
    }

    pub fn vlm_parse_error(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("AI 响应解析失败: {detail}");
        Self::VlmParseError { detail, cause: None, message }
    }

    pub fn vlm_low_confidence(confidence: f64) -> Self {
        let message = format!(
            "识别置信度过低 ({:.0}%)，请重新说出指令",
            confidence * 100.0
        );
        Self::VlmLowConfidence { confidence, message }
    }

    pub fn invalid_value(expected: impl Into<String>, got: u8) -> Self {
        let expected = expected.into();
        let message = format!("无效数值: 期望 {expected}，实际 {got}");
        Self::InvalidValue { expected, got, message }
    }

    pub fn device_not_found(device: impl Into<String>) -> Self {
        let device = device.into();
        let message = format!("设备未找到: {device}");
        Self::DeviceNotFound { device, message }
    }

    pub fn lock_error(lock_name: impl Into<String>) -> Self {
        let lock_name = lock_name.into();
        let message = format!("内部锁定失败: {lock_name}");
        Self::LockError { lock_name, message }
    }

    pub fn audio_device(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("音频设备错误: {detail}");
        Self::AudioDevice { detail, message }
    }

    pub fn camera_error(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("摄像头错误: {detail}");
        Self::CameraError { detail, message }
    }

    pub fn network_error(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("网络错误: {detail}");
        Self::NetworkError { detail, message }
    }

    pub fn python_env(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("Python 环境错误: {detail}");
        Self::PythonEnv { detail, message }
    }

    pub fn daemon_error(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("后端服务错误: {detail}");
        Self::DaemonError { detail, message }
    }

    pub fn gesture_error(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("手势识别错误: {detail}");
        Self::GestureError { detail, message }
    }

    pub fn internal_error(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let message = format!("系统内部错误: {detail}");
        Self::InternalError { detail, cause: None, message }
    }
}

// ============================================================
// From<T> 转换 —— 仅限孤儿规则允许的范围
// ============================================================

impl<T> From<std::sync::PoisonError<T>> for AppError {
    fn from(e: std::sync::PoisonError<T>) -> Self {
        Self::lock_error(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        let detail = e.to_string();
        let message = format!("AI 响应解析失败: {detail}");
        Self::VlmParseError { detail, cause: Some(Box::new(e)), message }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        let detail = e.to_string();
        let message = format!("系统内部错误: {detail}");
        Self::InternalError { detail, cause: Some(Box::new(e)), message }
    }
}
