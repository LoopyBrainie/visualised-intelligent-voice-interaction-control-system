// 可视化仿真区组件
export function VisualizationPanel() {
  return (
    <div class="w-full h-full flex items-center justify-center text-apple-text-tertiary">
      <div class="text-center space-y-2">
        <div class="w-16 h-16 mx-auto rounded-full bg-dark-surface-3 flex items-center justify-center">
          <svg class="w-8 h-8 text-apple-blue" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
        </div>
        <p class="text-sm">可视化仿真区</p>
        <p class="text-xs text-apple-text-tertiary/60">设备状态实时可视化</p>
      </div>
    </div>
  );
}
