"""errors.py - 代数式错误类型"""
from enum import StrEnum
from typing import Optional, Any
from pydantic import BaseModel


class ErrorCode(StrEnum):
    UNRECOGNIZED_COMMAND = "unrecognized_command"
    VLM_TIMEOUT = "vlm_timeout"
    VLM_PARSE_ERROR = "vlm_parse_error"
    VLM_LOW_CONFIDENCE = "vlm_low_confidence"
    INVALID_VALUE = "invalid_value"
    DEVICE_NOT_FOUND = "device_not_found"
    LOCK_ERROR = "lock_error"
    AUDIO_DEVICE = "audio_device"
    CAMERA_ERROR = "camera_error"
    NETWORK_ERROR = "network_error"
    PYTHON_ENV = "python_env"
    DAEMON_ERROR = "daemon_error"
    GESTURE_ERROR = "gesture_error"
    INTERNAL_ERROR = "internal_error"


class AppError(BaseModel):
    """纯数据模型 — 负责 JSON 序列化契约，与 Rust/TS 的 AppError 1:1 对齐"""
    code: ErrorCode
    message: str
    detail: Optional[str] = None


class AppException(Exception):
    """Python 原生异常 — 参与 raise/except 传播链，内部持有 AppError 数据"""
    def __init__(self, error: AppError):
        self.error = error
        super().__init__(error.message)


class AppResult(BaseModel):
    """HTTP 响应体 — success=True 时 data 有效，否则 error 有效"""
    success: bool
    data: Optional[Any] = None
    error: Optional[AppError] = None

    @classmethod
    def ok(cls, data: Any = None) -> "AppResult":
        return cls(success=True, data=data)

    @classmethod
    def fail(cls, code: ErrorCode, message: str, detail: Optional[str] = None) -> "AppResult":
        return cls(success=False, error=AppError(code=code, message=message, detail=detail))
