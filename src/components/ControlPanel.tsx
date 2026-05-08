import { createSignal } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';

// 控制按钮区组件
export function ControlPanel() {
  const [mode, setMode] = createSignal<'voice' | 'gesture'>('voice');
  const [isRunning, setIsRunning] = createSignal(false);

  const handleStart = async () => {
    try {
      await invoke('start_voice_capture');
      setIsRunning(true);
    } catch (e) {
      console.error('启动失败:', e);
    }
  };

  const handleStop = async () => {
    try {
      await invoke('stop_voice_capture');
      setIsRunning(false);
    } catch (e) {
      console.error('停止失败:', e);
    }
  };

  return (
    <div class="flex items-center gap-6">
      {/* 开始/停止按钮组 */}
      <div class="flex items-center gap-2">
        {/* 开始按钮 - Apple Blue CTA */}
        <button
          onClick={handleStart}
          disabled={isRunning()}
          class={`
            px-5 py-2 rounded-md text-sm font-medium transition-all duration-200
            focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-2 focus:ring-offset-primary
            ${isRunning()
              ? 'bg-surface-3 text-secondary cursor-not-allowed'
              : 'bg-accent text-white hover:bg-accent/90 active:scale-[0.98]'
            }
          `}
        >
          开始
        </button>

        {/* 停止按钮 - Secondary */}
        <button
          onClick={handleStop}
          disabled={!isRunning()}
          class={`
            px-5 py-2 rounded-md text-sm font-medium transition-all duration-200
            focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-2 focus:ring-offset-primary
            ${!isRunning()
              ? 'bg-surface-3 text-secondary cursor-not-allowed'
              : 'bg-surface-3 text-primary hover:bg-surface-4 active:scale-[0.98]'
            }
          `}
        >
          停止
        </button>
      </div>

      {/* 模式切换 - Apple 风格 Pill Toggle */}
      <div class="flex items-center gap-3">
        <span class="text-xs text-secondary">模式</span>
        <div class="flex bg-surface-3 rounded-full p-0.5">
          <button
            onClick={() => setMode('voice')}
            class={`
              px-4 py-1.5 text-xs font-medium rounded-full transition-all duration-200
              ${mode() === 'voice'
                ? 'bg-accent text-white shadow-sm'
                : 'text-secondary hover:text-primary'
              }
            `}
          >
            语音
          </button>
          <button
            onClick={() => setMode('gesture')}
            class={`
              px-4 py-1.5 text-xs font-medium rounded-full transition-all duration-200
              ${mode() === 'gesture'
                ? 'bg-accent text-white shadow-sm'
                : 'text-secondary hover:text-primary'
              }
            `}
          >
            手势
          </button>
        </div>
      </div>

      {/* 状态指示 */}
      <div class="flex items-center gap-2">
        <div
          class={`
            w-2 h-2 rounded-full transition-colors duration-300
            ${isRunning() ? 'bg-device-fan-on animate-pulse' : 'bg-surface-4'}
          `}
        />
        <span class="text-xs text-secondary">
          {isRunning() ? '运行中' : '已停止'}
        </span>
      </div>
    </div>
  );
}
