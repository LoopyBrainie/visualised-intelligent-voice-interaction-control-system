# gesture.py - 手势识别模块
# 职责: MediaPipe 手势关键点提取、手势分类
# MediaPipe 0.10.19+ 移除了 solutions API，使用 tasks API
import os
import cv2
import mediapipe as mp
import numpy as np
from mediapipe.tasks import python as mp_python
from mediapipe.tasks.python import vision

_MODEL_PATH = os.path.join(os.path.dirname(__file__), 'hand_landmarker.task')
_MODEL_URL = 'https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/latest/hand_landmarker.task'

# MediaPipe HandLandmarker 实例 (懒加载)
_landmarker = None


def _download_model():
    """首次运行时下载 HandLandmarker 模型文件 (~5MB)"""
    if not os.path.exists(_MODEL_PATH):
        import urllib.request
        import sys
        print(f"[Gesture] 正在下载 MediaPipe HandLandmarker 模型到 {_MODEL_PATH}...", file=sys.stderr)
        urllib.request.urlretrieve(_MODEL_URL, _MODEL_PATH)
        print(f"[Gesture] 模型下载完成", file=sys.stderr)
    return _MODEL_PATH


def _get_landmarker():
    """懒加载 MediaPipe HandLandmarker 实例（tasks API）"""
    global _landmarker
    if _landmarker is None:
        model_path = _download_model()
        base_options = mp_python.BaseOptions(model_asset_path=model_path)
        options = vision.HandLandmarkerOptions(
            base_options=base_options,
            num_hands=1,
            min_hand_detection_confidence=0.7,
            min_hand_presence_confidence=0.5,
        )
        _landmarker = vision.HandLandmarker.create_from_options(options)
    return _landmarker


# ============================================================
# 关键点索引定义 (MediaPipe Hands 21 关键点)
# ============================================================
class LM:  # Landmark
    WRIST = 0
    THUMB_CMC = 1
    THUMB_MCP = 2
    THUMB_IP = 3
    THUMB_TIP = 4
    INDEX_MCP = 5
    INDEX_PIP = 6
    INDEX_DIP = 7
    INDEX_TIP = 8
    MIDDLE_MCP = 9
    MIDDLE_PIP = 10
    MIDDLE_DIP = 11
    MIDDLE_TIP = 12
    RING_MCP = 13
    RING_PIP = 14
    RING_DIP = 15
    RING_TIP = 16
    PINKY_MCP = 17
    PINKY_PIP = 18
    PINKY_DIP = 19
    PINKY_TIP = 20


def _dist(p1, p2) -> float:
    """计算两个归一化坐标点之间的欧氏距离"""
    return np.sqrt((p1.x - p2.x) ** 2 + (p1.y - p2.y) ** 2)


# ============================================================
# 手指计数 (手掌尺寸归一化 + 拇指横向位移)
# ============================================================
#
# 旧实现使用 `tip_to_wrist > pip_to_wrist * 1.15` 的扁平阈值,
# 对小拇指尤其不友好 —— 小拇指的骨骼本身就比食指短,
# 同样的 1.15× 系数在它伸展时往往不够, 在它蜷曲时又容易过界。
#
# 新实现以手腕到中指 MCP 的距离作为"手掌尺寸"的参照,
# 把每根手指的"伸长量"用这个尺寸归一化,再用各自校准的系数。
# 这样阈值与摄像头距离、手的大小无关,只与"指尖相对指根伸出了多少"有关。

# 食指到小指: 指尖到手腕的距离必须比 pip 到手腕的距离多出 (k × 手掌尺寸)
# pinky 的 k 略低 —— 它的骨骼短,在伸展时绝对位移天然偏小
_FINGER_MIN_DELTA = {
    LM.INDEX_TIP:  0.45,
    LM.MIDDLE_TIP: 0.45,
    LM.RING_TIP:   0.45,
    LM.PINKY_TIP:  0.40,
}

# 拇指: 横向位移,基准点用 THUMB_MCP(因为它不朝手腕方向伸)
# 关键: 仅用"tip 离 MCP 距离 > IP 离 MCP 距离"这单一判据。
# 旧实现还要求 x 方向(右手 tip.x < ip.x + 0.05,左手反向),
# 但人手自然的微小旋转就会让 tip.x 越界,
# 把清晰的 5 指手势错算成 4 指。距离已经过 hand_size 归一化,
# 对"伸"与"屈"的区分度足够,方向校验只会带来假阴性。
_THUMB_MIN_DELTA = 0.30


def count_fingers(landmarks, handedness: str | None = None) -> tuple[int, float]:
    """
    计算伸出的手指数量

    食指~小指: 手掌尺寸归一化的"伸长量"超过各自阈值
    拇指: tip 相对 THUMB_MCP 的外伸距离超过 hand_size 加权的阈值
          (handedness 参数保留作未来扩展,当前不参与判定)

    Args:
        landmarks: MediaPipe 返回的 21 个 NormalizedLandmark
        handedness: "Left" 或 "Right",当前未使用(保留接口)

    Returns:
        (手指数量, 置信度)
    """
    wrist = landmarks[LM.WRIST]
    hand_size = _dist(wrist, landmarks[LM.MIDDLE_MCP])

    # 退化帧保护：MediaPipe 偶发返回接近 0 的关键点(理论上不应该发生)
    if hand_size < 1e-4:
        return 0, 0.0

    count = 0

    # --- 食指到小指: 手掌尺寸归一化 ---
    finger_pairs = [
        (LM.INDEX_TIP,  LM.INDEX_PIP),
        (LM.MIDDLE_TIP, LM.MIDDLE_PIP),
        (LM.RING_TIP,   LM.RING_PIP),
        (LM.PINKY_TIP,  LM.PINKY_PIP),
    ]

    for tip_idx, pip_idx in finger_pairs:
        tip = landmarks[tip_idx]
        pip = landmarks[pip_idx]
        delta = _dist(tip, wrist) - _dist(pip, wrist)
        if delta > _FINGER_MIN_DELTA[tip_idx] * hand_size:
            count += 1

    # --- 拇指: 仅距离判据,放弃方向校验(容差太紧在自然手姿下频繁假阴) ---
    thumb_tip = landmarks[LM.THUMB_TIP]
    thumb_ip  = landmarks[LM.THUMB_IP]
    thumb_mcp = landmarks[LM.THUMB_MCP]

    thumb_delta = _dist(thumb_tip, thumb_mcp) - _dist(thumb_ip, thumb_mcp)
    if thumb_delta > _THUMB_MIN_DELTA * hand_size:
        count += 1

    # --- 置信度 ---
    confidence = 0.85
    if count in (0, 5):
        confidence = 0.92
    elif count in (1, 4):
        confidence = 0.88

    return count, confidence


# ============================================================
# 手势识别入口
# ============================================================


def recognize_gesture(frame: np.ndarray) -> dict:
    """
    识别手势并返回伸出的手指数量与置信度

    wire format:
        {"count": int|None, "confidence": float, "landmarks": [...]}
    count 取值 0..5, 与手指伸出的数量一一对应 (0=握拳, 5=张开手掌)
    未检测到手时 count=None

    Args:
        frame: BGR 格式 numpy 数组 (OpenCV 默认)

    Returns:
        {"count": int|None, "confidence": float, "landmarks": list}
    """
    if frame is None or frame.size == 0:
        return {"count": None, "confidence": 0.0, "landmarks": []}

    detector = _get_landmarker()

    # BGR -> RGB，构建 MediaPipe Image
    rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
    mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb)
    result = detector.detect(mp_image)

    if not result.hand_landmarks:
        return {"count": None, "confidence": 0.0, "landmarks": []}

    # tasks API: hand_landmarks 已是 list[NormalizedLandmark]，无需 .landmark
    landmarks = result.hand_landmarks[0]

    # 获取左右手标签 (tasks API: category_name 对应 solutions API 的 label)
    handedness = None
    if result.handedness:
        handedness = result.handedness[0][0].category_name

    count, confidence = count_fingers(landmarks, handedness)

    landmark_list = [{"x": lm.x, "y": lm.y, "z": lm.z} for lm in landmarks]
    return {"count": count, "confidence": round(confidence, 2), "landmarks": landmark_list}
