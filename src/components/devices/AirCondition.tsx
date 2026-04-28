import { Component } from 'solid-js';
import { deviceState, setDeviceState } from '../../store/deviceStore';

export function AirCondition() {
  const toggleAC = () => {
    setDeviceState(prev => ({ ...prev, air_condition: !prev.air_condition }));
  };

  const setTemp = (value: number) => {
    setDeviceState(prev => ({ ...prev, ac_temp: value }));
  };

  return (
    <div class="flex items-center justify-between">
      {/* 开关 */}
      <div class="flex items-center gap-3">
        <span class="text-xs text-apple-text-secondary">空调</span>
        <button
          onClick={toggleAC}
          class={`
            relative w-11 h-6 rounded-full transition-colors duration-200
            ${deviceState().air_condition ? 'bg-apple-blue' : 'bg-dark-surface-4'}
          `}
        >
          <span
            class={`
              absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform duration-200
              ${deviceState().air_condition ? 'left-[22px]' : 'left-0.5'}
            `}
          />
        </button>
      </div>

      {/* 温度调节 */}
      <div class="flex items-center gap-3">
        <button
          onClick={() => setTemp(Math.max(16, deviceState().ac_temp - 1))}
          class="w-8 h-8 rounded-apple-sm bg-dark-surface-4 text-apple-text-secondary hover:text-apple-text-primary transition-colors duration-200 flex items-center justify-center"
        >
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
          </svg>
        </button>
        <span class="text-lg font-light text-apple-text-primary w-12 text-center">
          {deviceState().ac_temp}°
        </span>
        <button
          onClick={() => setTemp(Math.min(30, deviceState().ac_temp + 1))}
          class="w-8 h-8 rounded-apple-sm bg-dark-surface-4 text-apple-text-secondary hover:text-apple-text-primary transition-colors duration-200 flex items-center justify-center"
        >
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
        </button>
      </div>
    </div>
  );
}
