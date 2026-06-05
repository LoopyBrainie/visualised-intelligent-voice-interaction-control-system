"""
SPARV Python Daemon - VLM Audio Processing Service
Flask HTTP server for real-time audio → command conversion

B1: Flask HTTP skeleton (port 8765)
B2: /vlm endpoint with Base64 audio handling
B4: JSON response formatting with Pydantic validation

日志追踪:
- trace_id: 全链路追踪 ID，从 Rust 端传入
- 日志统一通过 /log 端点回传，由 Rust 转发到前端
"""
import os
import sys
import logging
import time
import threading
import traceback
import numpy as np
import cv2
from flask import Flask, request, jsonify
from pydantic import BaseModel, Field, ValidationError
from typing import Optional, List
from errors import ErrorCode, AppError, AppException, AppResult

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)
logger = logging.getLogger("daemon")

# Flask app
app = Flask(__name__)

# Configuration
HOST = "127.0.0.1"  # Local-only binding for security
PORT = 8765
MAX_CONTENT_LENGTH = 512 * 1024  # 512KB payload limit

# Import VLM engine
try:
    from ai_engine import call_vlm
    AI_ENGINE_AVAILABLE = True
except ImportError as e:
    logger.warning(f"ai_engine not available: {e}")
    AI_ENGINE_AVAILABLE = False

# Import gesture recognition
try:
    import gesture
    GESTURE_AVAILABLE = True
except ImportError as e:
    logger.warning(f"gesture module not available: {e}")
    GESTURE_AVAILABLE = False


# Pydantic models for request/response validation
class LogEntry(BaseModel):
    """日志条目"""
    timestamp: str = Field(description="时间戳 HH:MM:SS.mmm")
    level: str = Field(description="日志级别: DEBUG|INFO|WARN|ERROR")
    message: str = Field(description="日志消息")
    trace_id: Optional[str] = Field(default=None, description="追踪 ID")
    source: Optional[str] = Field(default=None, description="来源模块")


class VLMRequest(BaseModel):
    """VLM request payload"""
    audio: str = Field(description="Base64 encoded audio data with data:audio/pcm;base64, prefix")
    prompt: str = Field(default="请提取设备控制指令", description="Instruction prompt for VLM")
    trace_id: Optional[str] = Field(default=None, description="追踪 ID")
    sample_rate: int = Field(default=16000, description="音频原生采样率 (Hz)")


class VLMCommand(BaseModel):
    """Parsed command from VLM"""
    device: str = Field(description="Device name (light|fan|air_condition|curtain|all)")
    action: str = Field(description="Action (on|off|set_level|auto|stop|open|close)")
    target_value: int | None = Field(default=None, description="Target value for set_level")
    confidence: float = Field(ge=0.0, le=1.0, description="Confidence score")


class VLMResponse(BaseModel):
    """Successful VLM response"""
    command: VLMCommand
    raw_text: str = Field(description="Original command text")


def get_timestamp() -> str:
    """获取当前时间戳 HH:MM:SS.mmm"""
    t = time.time()
    hours = int(t // 3600) % 24
    minutes = int(t // 60) % 60
    seconds = int(t % 60)
    millis = int((t - int(t)) * 1000)
    return f"{hours:02}:{minutes:02}:{seconds:02}.{millis:03}"


def create_log(level: str, message: str, trace_id: Optional[str] = None, source: str = "daemon") -> dict:
    """创建日志条目"""
    return LogEntry(
        timestamp=get_timestamp(),
        level=level,
        message=message,
        trace_id=trace_id,
        source=source
    ).model_dump()


@app.errorhandler(AppException)
def handle_app_exception(e: AppException):
    """将 AppException 转为标准 AppResult JSON 响应"""
    return jsonify(AppResult(success=False, error=e.error).model_dump()), 500


@app.route("/health", methods=["GET"])
def health_check():
    """Health check endpoint for daemon monitoring"""
    return jsonify({"status": "ok"})


@app.route("/shutdown", methods=["POST", "GET"])
def shutdown():
    """Graceful shutdown endpoint"""
    logger.info("Shutdown signal received")
    func = request.environ.get("werkzeug.server.shutdown")
    if func is None:
        logger.warning("Werkzeug shutdown not available, exiting anyway")
        sys.exit(0)
    func()
    return jsonify({"status": "shutdown"})


@app.route("/vlm", methods=["POST"])
def vlm_endpoint():
    """
    VLM endpoint for audio → command conversion

    Request:
        POST /vlm
        Content-Type: application/json
        {"audio": "data:audio/pcm;base64,...", "prompt": "请提取设备控制指令", "trace_id": "abc123"}

    Response:
        Success: {"success": true, "data": {"command": {...}, "raw_text": "...", "logs": [...]}}
        Error:   {"success": false, "error": {"code": "...", "message": "...", "detail": "..."}}
    """
    logs = []  # 收集本次请求的日志

    # Parse and validate request
    try:
        data = VLMRequest(**request.get_json())
    except ValidationError as e:
        return jsonify(AppResult.fail(ErrorCode.INVALID_VALUE, str(e)).model_dump()), 400

    trace_id = data.trace_id
    logs.append(create_log("INFO", f"[VLM] 请求收到 trace_id={trace_id}", trace_id))

    # Check for empty audio
    if not data.audio or data.audio.strip() == "":
        logs.append(create_log("ERROR", "音频数据为空", trace_id))
        return jsonify(AppResult.fail(ErrorCode.INVALID_VALUE, "音频数据为空", detail=trace_id).model_dump()), 400

    # Check for empty prompt
    if not data.prompt or data.prompt.strip() == "":
        logs.append(create_log("ERROR", "prompt 为空", trace_id))
        return jsonify(AppResult.fail(ErrorCode.INVALID_VALUE, "无效请求: prompt为空", detail=trace_id).model_dump()), 400

    # Check if AI engine is available
    if not AI_ENGINE_AVAILABLE:
        logs.append(create_log("ERROR", "AI 引擎不可用", trace_id))
        return jsonify(AppResult.fail(ErrorCode.DAEMON_ERROR, "AI 引擎不可用", detail=trace_id).model_dump()), 503

    # Call VLM — 返回 AppResult 或 raise AppException（由 @app.errorhandler 捕获）
    try:
        logs.append(create_log("INFO", f"[VLM] 开始调用 AI 引擎 (sample_rate={data.sample_rate}Hz)", trace_id))
        result = call_vlm(data.audio, data.prompt, data.sample_rate)

        logs.append(create_log("INFO", f"[VLM] 调用完成 success={result.success}", trace_id))

        # result 是 AppResult — 直接使用其字段
        if not result.success:
            logs.append(create_log("ERROR", f"[VLM] 调用失败: {result.error.message if result.error else '未知'}", trace_id))
            return jsonify(result.model_dump()), 500

        # 注入日志到成功响应
        data_with_logs = dict(result.data or {})
        data_with_logs["logs"] = logs
        return jsonify(AppResult.ok(data=data_with_logs).model_dump())

    except AppException:
        raise  # 由 @app.errorhandler(AppException) 处理
    except Exception as e:
        logs.append(create_log("ERROR", f"[VLM] 未预期异常: {e}", trace_id))
        tb = "".join(traceback.format_exception(type(e), e, e.__traceback__))
        raise AppException(AppError(code=ErrorCode.INTERNAL_ERROR, message=str(e), detail=f"{trace_id}\n{tb}")) from e


@app.route("/gesture", methods=["POST"])
def gesture_endpoint():
    """
    手势识别端点

    Request:
        POST /gesture
        Content-Type: image/jpeg
        Body: raw JPEG bytes

    Response:
        Success: {"count": 0..5, "confidence": 0.85, "landmarks": [...]}
        No hand: {"count": null, "confidence": 0.0, "landmarks": []}
        Error:   {"success": false, "error": {"code": "...", "message": "...", "detail": "..."}}
    """
    if not GESTURE_AVAILABLE:
        return jsonify(AppResult.fail(ErrorCode.GESTURE_ERROR, "手势识别模块不可用").model_dump()), 503

    try:
        # 读取 raw JPEG bytes
        img_bytes = request.get_data()
        if not img_bytes:
            return jsonify(AppResult.fail(ErrorCode.INVALID_VALUE, "图像数据为空").model_dump()), 400

        # 解码 JPEG
        nparr = np.frombuffer(img_bytes, np.uint8)
        frame = cv2.imdecode(nparr, cv2.IMREAD_COLOR)

        if frame is None:
            return jsonify(AppResult.fail(ErrorCode.INVALID_VALUE, "图像解码失败，请确保数据为有效 JPEG").model_dump()), 400

        # 调用手势识别
        # 注意: 手势端点直接返回裸 dict，不做 AppResult 包装
        # Rust GestureClient 使用 response.json::<GestureResult>() 直接反序列化
        result = gesture.recognize_gesture(frame)
        return jsonify(result)

    except AppException:
        raise
    except Exception as e:
        logger.error(f"Gesture recognition error: {e}")
        tb = "".join(traceback.format_exception(type(e), e, e.__traceback__))
        raise AppException(AppError(code=ErrorCode.GESTURE_ERROR, message=str(e), detail=tb)) from e


def main():
    """Start the daemon server"""
    logger.info(f"Starting SPARV Daemon on {HOST}:{PORT}")
    if not AI_ENGINE_AVAILABLE:
        logger.warning("AI engine not available, /vlm endpoint will return errors")
    app.run(
        port=PORT,
        host=HOST,
        threaded=True,
        debug=False,
    )


if __name__ == "__main__":
    main()
