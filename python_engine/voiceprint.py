# voiceprint.py - 声纹识别模块 (预留接口) - 待实现
# 职责: MFCC 特征提取、声纹匹配校验
# 实验扩展功能，可后续集成
import numpy as np


def extract_mfcc(audio: np.ndarray, sample_rate: int = 16000) -> np.ndarray:
    """
    提取 MFCC 特征向量，用于声纹识别
    """
    # TODO: 实现MFCC特征提取
    pass


def verify_voiceprint(mfcc1: np.ndarray, mfcc2: np.ndarray, threshold: float = 10.0) -> bool:
    """
    欧氏距离校验声纹匹配
    """
    # TODO: 实现声纹校验
    pass
