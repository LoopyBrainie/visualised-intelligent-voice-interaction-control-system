# gesture.py - 手势识别模块 - 待实现
# 职责: MediaPipe 手势关键点提取、手势识别
import cv2
import mediapipe as mp
import numpy as np


mp_hands = mp.solutions.hands
hands = mp_hands.Hands(static_image_mode=False, max_hands=1)


def count_fingers(landmarks) -> int:
    """
    计算伸出的手指数量
    """
    # TODO: 实现手指计数
    pass


def recognize_gesture(frame: np.ndarray) -> str:
    """
    识别手势并返回类型
    -