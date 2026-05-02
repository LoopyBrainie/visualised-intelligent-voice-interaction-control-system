// spectrum.ts - 频谱事件订阅模块
// 职责: 接收后端 spectrum-update 事件，进行非线性动态范围压缩

import { listen, UnlistenFn } from '@tauri-apps/api/event';

// ============================================================
// 类型定义
// ============================================================

/** 后端发送的原始频谱数据 (512点 FFT) */
export interface SpectrumData {
  frequencies: number[];  // 512点 FFT 幅度 (归一化 0-1)
  sample_rate?: number;   // 原生采样率 Hz
  fft_size?: number;      // FFT 窗口大小
  timestamp?: number;     // 时间戳 ms
}

/** 频谱回调函数类型 (压缩后的 32 点数据) */
export type SpectrumCallback = (data: number[]) => void;

// ============================================================
// 常量
// ============================================================

const THROTTLE_MS = 33;   // 30Hz 节流 (60Hz 输入 → 30Hz 输出)
const TARGET_BARS = 32;    // 目标频谱柱数量

// 时间平滑
const SMOOTH_FACTOR = 0.6; // 平滑系数 [0-1], 越小越平滑

// ============================================================
// 模块状态
// ============================================================

let unlistenSpectrum: UnlistenFn | null = null;
let lastEmitTime = 0;
let lastSpectrum: number[] = Array(TARGET_BARS).fill(0);
let currentSampleRate: number = 0;

// ============================================================
// 频谱压缩 (512 → 32)
// ============================================================

/**
 * 将 512 点频谱压缩为 32 段
 * 1. 按频率段分组取平均
 * 2. 全局归一化 (以当前帧最大值为基准，保持相对关系)
 * 3. 时间平滑
 */
function compressSpectrum(full: number[]): number[] {
  // Step 1: 分段压缩 (512 → 32)
  const binSize = Math.floor(full.length / TARGET_BARS);
  const rawBars = Array.from({ length: TARGET_BARS }, (_, i) => {
    const start = i * binSize;
    const end = start + binSize;
    const slice = full.slice(start, end);
    return slice.reduce((a, b) => a + b, 0) / slice.length;
  });

  // Step 2: 全局归一化 (以当前帧最大值为基准)
  const frameMax = Math.max(...rawBars, 0.001); // 避免除零
  const normalized = rawBars.map(v => v / frameMax);

  // Step 3: 时间平滑 (减少抖动)
  const smoothed = normalized.map((v, i) =>
    v * (1 - SMOOTH_FACTOR) + lastSpectrum[i] * SMOOTH_FACTOR
  );
  lastSpectrum = smoothed;

  return smoothed;
}

// ============================================================
// 事件订阅
// ============================================================

/**
 * 订阅后端 spectrum-update 事件
 * @param callback 频谱数据回调（已压缩为 32 点）
 * @returns 取消订阅函数
 */
export async function subscribeSpectrum(callback: SpectrumCallback): Promise<UnlistenFn> {
  if (unlistenSpectrum) {
    unlistenSpectrum();
  }

  unlistenSpectrum = await listen<SpectrumData>('spectrum-update', (event) => {
    // 事件节流
    const now = Date.now();
    if (now - lastEmitTime < THROTTLE_MS) return;
    lastEmitTime = now;

    const payload = event.payload;
    const frequencies = Array.isArray(payload) ? payload : payload.frequencies;

    if (!frequencies || frequencies.length === 0) return;

    // 更新采样率元数据
    if (!Array.isArray(payload) && payload.sample_rate) {
      currentSampleRate = payload.sample_rate;
    }

    const compressed = compressSpectrum(frequencies);
    callback(compressed);
  });

  return unlistenSpectrum;
}

/**
 * 取消订阅 spectrum-update 事件
 */
export function unsubscribeSpectrum(): void {
  if (unlistenSpectrum) {
    unlistenSpectrum();
    unlistenSpectrum = null;
  }
}

/**
 * 获取当前音频采样率 (Hz)
 * 返回 0 表示尚未收到频谱数据
 */
export function getCurrentSampleRate(): number {
  return currentSampleRate;
}

/**
 * 创建本地模拟频谱数据（用于测试）
 */
export function createMockSpectrumData(bars: number = TARGET_BARS): number[] {
  return Array.from({ length: bars }, (_, i) => {
    const base = Math.sin((i / bars) * Math.PI) * 0.6 + 0.2;
    const noise = Math.random() * 0.3;
    return Math.min(1, Math.max(0.05, base + noise));
  });
}
