import { Component } from 'solid-js';
import { deviceState, setDeviceState } from '../../store/deviceStore';

export function Fan() {
  const toggleFan = () => {
    setDeviceState(prev => ({ ...prev, fan: !prev.fan }));
  };

  const setSpeed = (value: number) => {
    setDeviceState(prev => ({ ...prev, fan_speed: value }));
  };

  return (
    <div class="flex items-center justify-between">
      {/* 开关 */}
      <div class="flex items-center gap-3">
        <span class="text-xs text-apple-text-secondary">风扇</span>
        <button
          onClick={toggleFan}
          class={`
            relative w-11 h-6 rounded-full transition-colors duration-200
            ${deviceState().fan ? 'bg-apple-blue' : 'bg-dark-surface-4'}
          `}
        >
          <span
            class={`
              absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform duration-200
              ${deviceState().fan ? 'left-[22px]' : 'left-0.5'}
            `}
          />
        </button>
      </div>

      {/* 速度档位 */}
      <div class="flex items-center gap-2">
        {[1, 2, 3].map((speed) => (
          <button
            onClick={() => setSpeed(speed)}
            class={`
              w-8 h-8 rounded-apple-sm text-xs font-medium transition-colors duration-200
              ${deviceState().fan_speed === speed
                ? 'bg-apple-blue text-white'
                : 'bg-dark-surface-4 text-apple-text-tertiary hover:text-apple-text-secondary'}
            `}
          >
            {speed}
          </button>
        ))}
      </div>
    </div>
  );
}
