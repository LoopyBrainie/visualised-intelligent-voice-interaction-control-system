# ai_engine.py - AI 引擎主模块 - 待实现
# 职责: 音频降噪 + 手势识别的统一入口
import numpy as np


def process_audio(audio_data: np.ndarray) -> dict:
    """
    处理音频数据
    返回: {"filtered": np.ndarray, "spectrum": np.ndarray}
    """
    # TODO: 实现音频处理流程
    pass


def process_frame(frame: np.ndarray) -> str:
    """
    处理视频帧
    返回: 手势类型 ("palm", "fist", "peace", "pointing", "none")
    """
    # TODO: 实现帧处理流程
    pass
