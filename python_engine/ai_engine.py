"""
ai_engine.py - AI Engine for VLM Audio Processing
B3: VLM API calls with MiMo/Kimi compatible OpenAI SDK

Supports:
- MiMo API (Xiaomi)
- Kimi API (Moonshot)
- Standard OpenAI compatible APIs

Configuration: vlm_config.toml (do not use environment variables)
"""
import json
import logging
from pathlib import Path

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ai_engine")

# Find vlm_config.toml relative to this file
CONFIG_FILE = Path(__file__).parent / "vlm_config.toml"


def load_config() -> dict:
    """Load VLM configuration from vlm_config.toml"""
    try:
        import tomllib
        with open(CONFIG_FILE, "rb") as f:
            config = tomllib.load(f)
            return config.get("vlm", {})
    except ImportError:
        # Python < 3.11: try tomli
        try:
            import tomli
            with open(CONFIG_FILE, "rb") as f:
                config = tomli.load(f)
                return config.get("vlm", {})
        except ImportError:
            logger.warning("tomli not installed, using toml. Run: pip install tomli")
            import toml
            with open(CONFIG_FILE, "r") as f:
                config = toml.load(f)
                return config.get("vlm", {})
    except FileNotFoundError:
        logger.warning(f"Config file not found: {CONFIG_FILE}, using defaults")
        return {}
    except Exception as e:
        logger.warning(f"Failed to load config: {e}, using defaults")
        return {}


# Load config at module import
_config = load_config()

# VLM API Configuration from TOML (with defaults)
VLM_BASE_URL = _config.get("base_url", "https://api.xiaomimimo.com/v1")
VLM_API_KEY = _config.get("api_key", "your-api-key-here")
VLM_MODEL = _config.get("model", "mimo-v2.5")
VLM_TIMEOUT = _config.get("timeout", 10.0)
VLM_MAX_RETRIES = _config.get("max_retries", 3)
VLM_MAX_COMPLETION_TOKENS = _config.get("max_completion_tokens", 256)

# System prompt for smart home voice command parsing
SYSTEM_PROMPT = """你是一个智能家居语音指令解析器。请分析用户音频，理解用户的操作意图，提取设备控制指令。

**输出格式** (仅返回 JSON，不要任何其他文字):
{
    "device": "设备名称(light|fan|air_condition|curtain)",
    "action": "动作(on|off|set_level|auto|stop|open|close)",
    "room": "房间名称(living_room|bedroom|all)，如果用户明确指定了房间则填写，否则为null",
    "target_value": 数值(可选,仅set_level时，如灯光亮度0-100，空调温度16-30，风速1-3),
    "confidence": 0.0-1.0置信度
}

**设备指令映射**:
- light: on/off/set_level(亮度0-100)
- fan: on/off/set_level(风速1-3)
- air_condition: on/off/set_level(温度16-30)/auto
- curtain: open/close/stop

**房间识别规则**:
- "客厅"、"客厅的" → living_room
- "卧室"、"卧房"、"睡房"、"房间" → bedroom
- 未明确指定房间 → null（让Rust端根据上下文决策）
如果你判断有可能是空音频，应当输出较低的可置信度，当可置信度小于0.6时，指令会被忽略

**示例**:
音频"打开客厅灯" → {"device":"light","action":"on","room":"living_room","confidence":0.95}
音频"把空调调到25度" → {"device":"air_condition","action":"set_level","target_value":25,"room":null,"confidence":0.90}
音频"关闭卧室风扇" → {"device":"fan","action":"off","room":"bedroom","confidence":0.95}
音频"拉开窗帘" → {"device":"curtain","action":"open","room":null,"confidence":0.92}
音频"打开所有灯" → {"device":"light","action":"on","room":"all","confidence":0.88}


仅返回 JSON，不要 markdown 代码块。
"""


def get_client():
    """Get or create OpenAI client (lazy initialization)"""
    try:
        from openai import OpenAI
        return OpenAI(
            api_key=VLM_API_KEY,
            base_url=VLM_BASE_URL,
            timeout=VLM_TIMEOUT,
        )
    except ImportError:
        logger.error("OpenAI SDK not installed. Run: pip install openai")
        raise


def parse_audio_prefix(audio_b64: str) -> tuple[str, str]:
    """
    Parse audio string to extract MIME type and raw base64 data.

    Args:
        audio_b64: Audio string, possibly with "data:audio/pcm;base64," prefix

    Returns:
        (mime_type, raw_base64_data)
    """
    if audio_b64.startswith("data:"):
        # Extract MIME type and base64 data
        parts = audio_b64.split(",", 1)
        if len(parts) == 2:
            header = parts[0]  # e.g., "data:audio/pcm;base64"
            data = parts[1]
            # Extract MIME type from header
            mime_type = header.split(";")[0].replace("data:", "")
            return mime_type, data
        elif len(parts) == 1:
            # No comma, treat entire string as raw base64
            return "audio/pcm", audio_b64
    # No prefix, treat as raw base64
    return "audio/pcm", audio_b64


def encode_pcm_to_wav(pcm_data: bytes, sample_rate: int = 16000, channels: int = 1, bits: int = 16) -> bytes:
    """
    Encode raw PCM data to WAV format.

    Args:
        pcm_data: Raw PCM bytes (16-bit, mono, little-endian)
        sample_rate: Sample rate in Hz (default 16000)
        channels: Number of channels (default 1 for mono)
        bits: Bits per sample (default 16)

    Returns:
        WAV encoded bytes
    """
    import wave
    import io
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(channels)
        w.setsampwidth(bits // 8)  # bytes per sample
        w.setframerate(sample_rate)
        w.writeframes(pcm_data)
    return buf.getvalue()


def call_vlm(audio_b64: str, prompt: str = "提取设备控制指令", sample_rate: int = 16000) -> dict:
    """
    Call VLM API to convert audio to command.

    Args:
        audio_b64: Base64 encoded audio data, possibly with data:audio/pcm;base64, prefix
        prompt: Text prompt to guide the VLM
        sample_rate: Audio native sample rate in Hz (default 16000)

    Returns:
        dict with keys:
            - success: bool
            - command: VLMCommand dict if success, None otherwise
            - raw_text: str description if success, error message otherwise
            - error: str error type if failed, None otherwise
    """
    import signal
    import base64 as b64

    # Timeout handler for VLM API calls (Unix only)
    def timeout_handler(signum, frame):
        raise TimeoutError("VLM API 调用超时 (12s)")

    # 设置 12 秒超时（略短于 Rust 端的 15 秒超时）
    timeout_set = False
    try:
        signal.signal(signal.SIGALRM, timeout_handler)
        signal.alarm(12)
        timeout_set = True
    except (AttributeError, OSError):
        # Windows 不支持 SIGALRM，使用线程超时替代
        pass

    try:
        # Parse audio prefix
        mime_type, raw_audio = parse_audio_prefix(audio_b64)

        # Validate audio data
        if not raw_audio or raw_audio.strip() == "":
            return {
                "success": False,
                "command": None,
                "raw_text": "",
                "error": "empty_audio",
                "message": "音频数据为空",
            }

        # Decode base64 to raw PCM bytes
        try:
            pcm_bytes = b64.b64decode(raw_audio)
        except Exception as e:
            return {
                "success": False,
                "command": None,
                "raw_text": "",
                "error": "decode_error",
                "message": f"Base64 解码失败: {str(e)}",
            }

        # Encode PCM to WAV (API only supports encoded formats, not raw PCM)
        try:
            wav_bytes = encode_pcm_to_wav(pcm_bytes, sample_rate=sample_rate, channels=1, bits=16)
            wav_b64 = b64.b64encode(wav_bytes).decode("ascii")
            # Use audio/wav for the API
            mime_type = "audio/wav"
        except Exception as e:
            return {
                "success": False,
                "command": None,
                "raw_text": "",
                "error": "wav_encode_error",
                "message": f"WAV 编码失败: {str(e)}",
            }

        # Get client
        try:
            client = get_client()
        except Exception as e:
            return {
                "success": False,
                "command": None,
                "raw_text": "",
                "error": "client_error",
                "message": f"无法初始化 VLM 客户端: {str(e)}",
            }

        # Build messages
        messages = [
            {"role": "system", "content": SYSTEM_PROMPT},
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_audio",
                        "input_audio": {
                            "data": f"data:{mime_type};base64,{wav_b64}"
                        }
                    },
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            }
        ]

        # Call API with retries
        max_retries = VLM_MAX_RETRIES
        last_error = None

        for attempt in range(max_retries):
            try:
                completion = client.chat.completions.create(
                    model=VLM_MODEL,
                    messages=messages,
                    response_format={"type": "json_object"},  # JSON Mode
                    max_completion_tokens=VLM_MAX_COMPLETION_TOKENS,
                    temperature=0.8,  # Low temperature for consistent JSON
                )

                # Parse response
                content = completion.choices[0].message.content

                # Handle case where content might be wrapped in code blocks
                if content.startswith("```json"):
                    content = content[7:]
                if content.startswith("```"):
                    content = content[3:]
                if content.endswith("```"):
                    content = content[:-3]

                result = json.loads(content.strip())

                # Validate result structure
                if "device" in result and "action" in result:
                    return {
                        "success": True,
                        "command": {
                            "device": result.get("device", "light"),
                            "action": result.get("action", "on"),
                            "room": result.get("room"),  # 可能为 null
                            "target_value": result.get("target_value"),
                            "confidence": result.get("confidence", 0.5),
                        },
                        "raw_text": f"{result.get('device', 'unknown')}-{result.get('action', 'unknown')}-{result.get('room', 'none')}",
                        "error": None,
                        "message": "success",
                    }
                else:
                    return {
                        "success": False,
                        "command": None,
                        "raw_text": content[:100] if content else "",
                        "error": "parse_error",
                        "message": f"VLM 响应格式无效，缺少必要字段: {result}",
                    }

            except TimeoutError:
                last_error = "VLM API 调用超时 (12s)"
                logger.warning(f"Attempt {attempt + 1}: VLM API 超时")
                break  # 不重试超时

            except json.JSONDecodeError as e:
                last_error = f"JSON 解析失败: {str(e)}"
                logger.warning(f"Attempt {attempt + 1}: JSON decode error: {e}")

            except Exception as e:
                last_error = f"API 调用失败: {str(e)}"
                logger.warning(f"Attempt {attempt + 1}: {e}")

            # Wait before retry (exponential backoff)
            if attempt < max_retries - 1:
                import time
                time.sleep(0.5 * (2 ** attempt))

        # All retries failed
        return {
            "success": False,
            "command": None,
            "raw_text": "",
            "error": "api_error",
            "message": f"VLM API 调用失败，已重试 {max_retries} 次: {last_error}",
        }

    finally:
        # 取消 alarm
        if timeout_set:
            signal.alarm(0)


# Test function
if __name__ == "__main__":
    # Test with a dummy audio
    print(f"Config loaded from: {CONFIG_FILE}")
    print(f"Base URL: {VLM_BASE_URL}")
    print(f"Model: {VLM_MODEL}")
    test_result = call_vlm("data:audio/pcm;base64,AAAA", "打开灯")
    print(json.dumps(test_result, ensure_ascii=False, indent=2))