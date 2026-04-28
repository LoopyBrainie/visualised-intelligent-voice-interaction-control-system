import { Component, onMount, onCleanup, createSignal } from 'solid-js';

export function SpectrumPanel() {
  const [isActive, setIsActive] = createSignal(false);
  let animationRef: number;

  onMount(() => {
    // 模拟频谱动画
    const animate = () => {
      setIsActive(prev => !prev); // 简单切换用于视觉反馈
      animationRef = requestAnimationFrame(animate);
    };
    // 延迟启动动画
    const timeout = setTimeout(() => {
      animate();
    }, 500);
    onCleanup(() => {
      clearTimeout(timeout);
      cancelAnimationFrame(animationRef);
    });
  });

  return (
    <div class="h-full flex flex-col">
      {/* 标题栏 */}
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center gap-2">
          <div class={`w-2 h-2 rounded-full transition-colors duration-300 ${isActive() ? 'bg-apple-blue animate-pulse' : 'bg-apple-text-tertiary'}`} />
          <h3 class="text-sm font-semibold text-apple-text-primary tracking-tight">
            音频频谱
          </h3>
        </div>
        <span class="text-xs text-apple-text-tertiary">波形</span>
      </div>

      {/* 频谱可视化区域 */}
      <div class="flex-1 flex items-end justify-center gap-1 px-2">
        {/* 生成频谱柱状条 */}
        {Array.from({ length: 32 }, (_, i) => {
          // 模拟不同高度的频谱条
          const baseHeight = Math.sin(i * 0.3) * 30 + 40;
          const randomVariation = Math.random() * 20;
          const height = isActive() ? baseHeight + randomVariation : 8;

          return (
            <div
              class="w-1.5 rounded-t transition-all duration-75"
              style={{
                height: `${height}%`,
                background: isActive()
                  ? `linear-gradient(to top, #0071e3 ${100 - (i % 4) * 20}%, #2997ff)`
                  : '#3a3a3c',
              }}
            />
          );
        })}
      </div>

      {/* 底部信息 */}
      <div class="flex items-center justify-between mt-2 pt-2 border-t border-white/[0.05]">
        <span class="text-[10px] text-apple-text-tertiary">采样率: 16kHz</span>
        <span class="text-[10px] text-apple-text-tertiary">实时</span>
      </div>
    </div>
  );
}
