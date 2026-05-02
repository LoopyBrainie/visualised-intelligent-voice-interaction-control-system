import { createSignal, onCleanup, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LiquidGlassSegmentToggle } from './ui/LiquidGlassSegmentToggle';
import { MicButton } from './ui/MicButton';
import { LiquidGlass } from './ui/LiquidGlass';

interface ControlBarProps {
  isPortrait: boolean;
}

export function ControlBar(_props: ControlBarProps) {
  const [mode, setMode] = createSignal<'voice' | 'gesture'>('voice');
  const [isProcessing, setIsProcessing] = createSignal(false);

  // 初始化同步后端状态 + 监听事件
  onMount(() => {
    invoke('set_language_mode', { enabled: mode() === 'voice' }).catch(console.error);

    const unlisten = listen<boolean>('vlm-processing', (event) => {
      setIsProcessing(event.payload);
    });
    onCleanup(() => { unlisten.then(fn => fn()); });
  });

  // 语言模式切换时通知后端
  const handleModeChange = (checked: boolean) => {
    const newMode = checked ? 'gesture' : 'voice';
    setMode(newMode);
    // 通知后端语言模式状态（语言模式 = voice）
    invoke('set_language_mode', { enabled: newMode === 'voice' }).catch(console.error);
  };

  return (
    <div class="fixed bottom-4 left-4 right-4 z-control-bar" style={{ height: '72px' }}>
      <LiquidGlass
        radius={16}
        blur={5}
        background="rgba(255, 255, 255, 0.3)"
        contrast={1.25}
        brightness={1.05}
        saturate={1.15}
        overlay={true}
        displacement={false}
        style={{ width: '100%', height: '100%' }}
      >
      <div
        class="relative z-10 w-full h-full flex flex-row items-center justify-between gap-4 px-4 py-3"
      >
        {/* 麦克风按钮 - 左侧 */}
        <MicButton size={52} />

        {/* 右侧：语言/手势 toggle */}
        <div class="flex items-center gap-3">
          <LiquidGlassSegmentToggle
            leftLabel="语言"
            rightLabel="手势"
            checked={mode() === 'gesture'}
            onChange={handleModeChange}
          />
        </div>
      </div>
      </LiquidGlass>
    </div>
  );
}
