import { Component, onMount, onCleanup, createSignal } from 'solid-js';
import { subscribeSpectrum, unsubscribeSpectrum, createMockSpectrumData, getCurrentSampleRate } from '../lib/spectrum';

export function SpectrumPanel() {
  // 真实频谱数据 (32 点)
  const [spectrumData, setSpectrumData] = createSignal<number[]>(Array(32).fill(0.05));
  const [isListening, setIsListening] = createSignal(false);
  const [sampleRate, setSampleRate] = createSignal(0);

  onMount(async () => {
    // 尝试订阅真实频谱数据
    try {
      await subscribeSpectrum((data) => {
        setSpectrumData(data);
        if (!isListening()) setIsListening(true);
        // 同步采样率元数据
        const sr = getCurrentSampleRate();
        if (sr > 0 && sampleRate() !== sr) setSampleRate(sr);
      });
    } catch (e) {
      // 降级到模拟数据
      startMockAnimation();
    }
  });

  onCleanup(() => {
    unsubscribeSpectrum();
  });

  // 模拟数据动画（当真实数据不可用时）
  let mockInterval: number;
  function startMockAnimation() {
    mockInterval = window.setInterval(() => {
      setSpectrumData(createMockSpectrumData(32));
    }, 33); // ~30Hz
  }

  return (
    <div class="h-full flex flex-col">
      {/* 标题栏 */}
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center gap-2">
          <div class={`w-2 h-2 rounded-full transition-colors duration-300 ${isListening() ? 'bg-apple-blue animate-pulse' : 'bg-apple-text-tertiary'}`} />
          <h3 class="text-sm font-semibold text-apple-text-primary tracking-tight">
            音频频谱
          </h3>
        </div>
        <span class="text-xs text-apple-text-tertiary">
          {isListening() ? '实时' : '待机'}
        </span>
      </div>

      {/* 频谱可视化区域 - GPU 加速 */}
      <div class="flex-1 flex items-end justify-center gap-1 px-2">
        {Array.from({ length: 32 }, (_, i) => {
          // 使用 scaleY 进行 GPU 加速渲染，但需要 h-full 作为基准高度
          const scale = spectrumData()[i] || 0.05;
          const isActive = scale > 0.1;

          return (
            <div
              class="w-1.5 h-full rounded-t"
              style={{
                transform: `scaleY(${scale})`,
                "transform-origin": "bottom",
                background: isActive ? '#0071e3' : '#3a3a3c',
                transition: 'transform 0.15s cubic-bezier(0.4, 0, 0.2, 1)',
              }}
            />
          );
        })}
      </div>

      {/* 底部信息 */}
      <div class="flex items-center justify-between mt-2 pt-2 border-t border-white/[0.05]">
        <span class="text-[10px] text-apple-text-tertiary">
          {sampleRate() > 0 ? `采样率: ${(sampleRate() / 1000).toFixed(1)}kHz` : '采样率: --'}
        </span>
        <span class="text-[10px] text-apple-text-tertiary">FFT: 512pt</span>
      </div>
    </div>
  );
}