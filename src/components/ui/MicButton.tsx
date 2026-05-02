import { createSignal, onCleanup, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LiquidGlass } from './LiquidGlass';

interface MicButtonProps {
  size?: number;
}

export function MicButton(props: MicButtonProps) {
  const size = props.size ?? 56;
  const [isRunning, setIsRunning] = createSignal(false);
  const [isProcessing, setIsProcessing] = createSignal(false);

  onMount(() => {
    const unlisten = listen<boolean>('vlm-processing', (event) => {
      setIsProcessing(event.payload);
    });
    onCleanup(() => { unlisten.then(fn => fn()); });
  });

  const hasTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  const handleClick = async () => {
    const next = !isRunning();
    setIsRunning(next);
    if (hasTauri()) {
      try {
        await invoke(next ? 'start_voice_capture' : 'stop_voice_capture');
      } catch (e) {
        console.error('麦克风切换失败:', e);
        setIsRunning(!next);
      }
    }
  };

  const bg = () => isRunning() ? 'rgba(0, 113, 227, 0.6)' : 'rgba(255, 255, 255, 0.8)';

  return (
    <button
      onClick={handleClick}
      class="cursor-pointer transition-transform duration-300 active:scale-95"
      style={{ width: `${size}px`, height: `${size}px` }}
    >
      <LiquidGlass
        radius={size / 2}
        blur={5}
        background={bg()}
        contrast={1.15}
        brightness={isRunning() ? 1.1 : 1.05}
        saturate={isRunning() ? 1.3 : 1.1}
        overlay={true}
        style={{
          width: `${size}px`,
          height: `${size}px`,
          display: 'flex',
          'align-items': 'center',
          'justify-content': 'center',
        }}
      >
        <svg
          width={size * 0.45}
          height={size * 0.45}
          viewBox="0 0 24 24"
          fill="none"
          stroke={isRunning() ? '#ffffff' : 'rgba(30,30,30,0.8)'}
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="transition-colors duration-300"
        >
          <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
          <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
          <line x1="12" x2="12" y1="19" y2="22" />
        </svg>

        {isProcessing() && (
          <div
            class="absolute inset-0 rounded-full"
            style={{
              border: '3px solid transparent',
              'border-top-color': 'rgba(255,255,255,0.7)',
              animation: 'spin 0.8s linear infinite',
            }}
          />
        )}
      </LiquidGlass>
    </button>
  );
}
