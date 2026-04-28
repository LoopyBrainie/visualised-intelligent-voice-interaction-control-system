// 控制按钮区组件
export function ControlPanel() {
  return (
    <div class="flex items-center gap-4">
      {/* 开始按钮 */}
      <button class="px-6 py-2.5 bg-apple-blue text-white rounded-apple-md font-medium text-sm hover:bg-apple-blue-hover active:scale-95 transition-all focus:outline-none focus:ring-2 focus:ring-apple-blue focus:ring-offset-2 focus:ring-offset-morandi-dark">
        开始
      </button>

      {/* 停止按钮 */}
      <button class="px-6 py-2.5 bg-dark-surface-3 text-apple-text-primary rounded-apple-md font-medium text-sm hover:bg-dark-surface-4 active:scale-95 transition-all focus:outline-none focus:ring-2 focus:ring-apple-blue focus:ring-offset-2 focus:ring-offset-morandi-dark">
        停止
      </button>

      {/* 模式切换 */}
      <div class="flex items-center gap-2 ml-4">
        <span class="text-xs text-apple-text-tertiary">模式:</span>
        <div class="flex bg-dark-surface-3 rounded-apple-md p-0.5">
          <button class="px-3 py-1.5 text-xs bg-apple-blue text-white rounded-apple-sm transition-colors">
            语音
          </button>
          <button class="px-3 py-1.5 text-xs text-apple-text-secondary hover:text-apple-text-primary rounded-apple-sm transition-colors">
            手势
          </button>
        </div>
      </div>
    </div>
  );
}
