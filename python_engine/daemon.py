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
from flask import Flask, request, jsonify
from pydantic import BaseModel, Field, ValidationError
from typing import Optional, List

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


class ErrorResponse(BaseModel):
    """Error response"""
    error: str = Field(description="Error type")
    command: None = Field(default=None)
    message: str = Field(description="Error message")
    trace_id: Optional[str] = Field(default=None, description="追踪 ID")


def make_error_response(error_type: str, message: str, trace_id: Optional[str] = None) -> dict:
    """Create standardized error response"""
    return ErrorResponse(error=error_type, command=None, message=message, trace_id=trace_id).model_dump()


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
        Success: {"command": {"device": "...", "action": "...", ...}, "raw_text": "...", "logs": [...]}
        Error: {"error": "error_type", "command": null, "message": "...", "trace_id": "..."}
    """
    logs = []  # 收集本次请求的日志

    # Parse and validate request
    try:
        data = VLMRequest(**request.get_json())
    except ValidationError as e:
        return jsonify(make_error_response("validation_error", str(e))), 400

    trace_id = data.trace_id
    logs.append(create_log("INFO", f"[VLM] 请求收到 trace_id={trace_id}", trace_id))

    # Check for empty audio
    if not data.audio or data.audio.strip() == "":
        logs.append(create_log("ERROR", "音频数据为空", trace_id))
        return jsonify(make_error_response("empty_audio", "音频数据为空", trace_id)), 400

    # Check for empty prompt
    if not data.prompt or data.prompt.strip() == "":
        logs.append(create_log("ERROR", "prompt 为空", trace_id))
        return jsonify(make_error_response("invalid_request", "无效请求: prompt为空", trace_id)), 400

    # Check if AI engine is available
    if not AI_ENGINE_AVAILABLE:
        logs.append(create_log("ERROR", "AI 引擎不可用", trace_id))
        return jsonify(make_error_response("engine_unavailable", "AI 引擎不可用", trace_id)), 503

    # Call VLM
    try:
        logs.append(create_log("INFO", f"[VLM] 开始调用 AI 引擎 (sample_rate={data.sample_rate}Hz)", trace_id))
        result = call_vlm(data.audio, data.prompt, data.sample_rate)

        logs.append(create_log("INFO", f"[VLM] 调用完成 success={result.get('success', False)}", trace_id))

        # Check if VLM call was successful
        if not result.get("success", False):
            error_msg = result.get("message", "VLM 调用失败")
            logs.append(create_log("ERROR", f"[VLM] 调用失败: {error_msg}", trace_id))
            return jsonify(make_error_response(
                result.get("error", "unknown"),
                error_msg,
                trace_id
            )), 500

        # 返回成功响应（带日志）
        response = {
            "command": result.get("command"),
            "raw_text": result.get("raw_text", ""),
            "logs": logs  # 回传日志到 Rust
        }
        return jsonify(response)

    except Exception as e:
        logs.append(create_log("ERROR", f"VLM call failed: {e}", trace_id))
        return jsonify(make_error_response("vlm_error", f"VLM 调用失败: {str(e)}", trace_id)), 500


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
