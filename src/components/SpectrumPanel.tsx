// 音频频谱区组件
export function SpectrumPanel() {
  return (
    <div class="w-full h-full flex items-center justify-center text-apple-text-tertiary">
      <div class="text-center space-y-2">
        <div class="w-16 h-16 mx-auto rounded-full bg-dark-surface-3 flex items-center justify-center">
          <svg class="w-8 h-8 text-apple-blue" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
          </svg>
        </div>
        <p class="text-sm">音频频谱区</p>
        <p class="text-xs text-apple-text-tertiary/60">实时音频波形</p>
      </div>
    </div>
  );
}
