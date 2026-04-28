# dsp_processor.py - 音频 DSP 处理模块 - 待实现
# 职责: scipy 降噪滤波、FFT 频谱计算
import numpy as np
from typing import List


def apply_filter(audio: np.ndarray) -> np.ndarray:
    """
    FIR 滤波器降噪: y[n] = Σbk·x[n-k]
    使用均值滤波器 b = [1/M, 1/M, ..., 1/M]
    """
    # TODO: 实现均值滤波
    pass


def get_spectrum(audio: np.ndarray) -> np.ndarray:
    """
    返回 FFT 频谱幅度 (dB)
    取512点进行快速傅里叶变换
    """
    # TODO: 实现FFT频谱计算
    pass
