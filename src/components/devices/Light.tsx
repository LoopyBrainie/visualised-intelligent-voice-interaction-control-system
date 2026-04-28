import { Component } from 'solid-js';
import { deviceState, setDeviceState } from '../../store/deviceStore';

export function Light() {
  const toggleLight = () => {
    setDeviceState(prev => ({ ...prev, light: !prev.light }));
  };

  const setBrightness = (value: number) => {
    setDeviceState(prev => ({ ...prev, light_brightness: value }));
  };

  return (
    <div class="flex items-center justify-between">
      {/* 开关 */}
      <div class="flex items-center gap-3">
        <span class="text-xs text-apple-text-secondary">灯光</span>
        <button
          onClick={toggleLight}
          class={`
            relative w-11 h-6 rounded-full transition-colors duration-200
            ${deviceState().light ? 'bg-apple-blue' : 'bg-dark-surface-4'}
          `}
        >
          <span
            class={`
              absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform duration-200
              ${deviceState().light ? 'left-[22px]' : 'left-0.5'}
            `}
          />
        </button>
      </div>

      {/* 亮度滑块 */}
      <div class="flex items-center gap-3">
        <span class="text-xs text-apple-text-tertiary">{deviceState().light_brightness}%</span>
        <input
          type="range"
          min="0"
          max="100"
          value={deviceState().light_brightness}
          onInput={(e) => setBrightness(parseInt(e.currentTarget.value))}
          class="w-24 h-1 bg-dark-surface-4 rounded-full appearance-none cursor-pointer
            [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3
            [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-apple-blue
            [&::-webkit-slider-thumb]:cursor-pointer"
        />
      </div>
    </div>
  );
}
