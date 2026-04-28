import { Component, createSignal, Show } from 'solid-js';
import { Light } from './devices/Light';
import { Fan } from './devices/Fan';
import { AirCondition } from './devices/AirCondition';
import { deviceState } from '../store/deviceStore';

export function VisualizationPanel() {
  const [activeDevice, setActiveDevice] = createSignal<'light' | 'fan' | 'ac' | null>(null);

  return (
    <div class="h-full flex flex-col">
      {/* 设备选择器标签 */}
      <div class="flex items-center gap-2 mb-4">
        <div class="w-2 h-2 rounded-full bg-apple-blue animate-pulse" />
        <h3 class="text-sm font-semibold text-apple-text-primary tracking-tight">
          设备控制中心
        </h3>
      </div>

      {/* 设备卡片网格 */}
      <div class="flex-1 grid grid-cols-3 gap-4">
        {/* 灯光设备卡片 */}
        <div
          onClick={() => setActiveDevice(activeDevice() === 'light' ? null : 'light')}
          class={`
            relative cursor-pointer rounded-apple-md border transition-all duration-200
            ${deviceState().light
              ? 'bg-apple-blue/10 border-apple-blue/30'
              : 'bg-dark-surface-3/50 border-white/[0.05] hover:border-white/[0.1]'}
          `}
        >
          <div class="p-4 flex flex-col items-center justify-center h-full gap-3">
            {/* 设备图标 */}
            <div class={`
              w-12 h-12 rounded-full flex items-center justify-center transition-colors
              ${deviceState().light ? 'bg-apple-blue/20' : 'bg-dark-surface-4'}
            `}>
              <svg class={`w-6 h-6 ${deviceState().light ? 'text-apple-blue' : 'text-apple-text-tertiary'}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
              </svg>
            </div>
            {/* 设备名称 */}
            <span class="text-xs font-medium text-apple-text-secondary">灯光</span>
            {/* 状态指示 */}
            <span class={`text-[10px] ${deviceState().light ? 'text-apple-blue' : 'text-apple-text-tertiary'}`}>
              {deviceState().light ? `亮度 ${deviceState().light_brightness}%` : '关闭'}
            </span>
          </div>
          {/* 开启状态指示条 */}
          <Show when={deviceState().light}>
            <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-apple-blue rounded-b-apple-md" />
          </Show>
        </div>

        {/* 风扇设备卡片 */}
        <div
          onClick={() => setActiveDevice(activeDevice() === 'fan' ? null : 'fan')}
          class={`
            relative cursor-pointer rounded-apple-md border transition-all duration-200
            ${deviceState().fan
              ? 'bg-apple-blue/10 border-apple-blue/30'
              : 'bg-dark-surface-3/50 border-white/[0.05] hover:border-white/[0.1]'}
          `}
        >
          <div class="p-4 flex flex-col items-center justify-center h-full gap-3">
            {/* 设备图标 */}
            <div class={`
              w-12 h-12 rounded-full flex items-center justify-center transition-colors
              ${deviceState().fan ? 'bg-apple-blue/20' : 'bg-dark-surface-4'}
            `}>
              <svg class={`w-6 h-6 ${deviceState().fan ? 'text-apple-blue' : 'text-apple-text-tertiary'}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 12c-1.5 0-2.5 1-3 2-.5-1-1.5-2-3-2-2 0-3.5 1.5-3.5 3.5 0 1.5.8 2.5 2 3.5V19a2 2 0 002 2h5a2 2 0 002-2v-1.5c1.2-1 2-2 2-3.5 0-2-1.5-3.5-3.5-3.5z" />
              </svg>
            </div>
            {/* 设备名称 */}
            <span class="text-xs font-medium text-apple-text-secondary">风扇</span>
            {/* 状态指示 */}
            <span class={`text-[10px] ${deviceState().fan ? 'text-apple-blue' : 'text-apple-text-tertiary'}`}>
              {deviceState().fan ? `档位 ${deviceState().fan_speed}` : '关闭'}
            </span>
          </div>
          <Show when={deviceState().fan}>
            <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-apple-blue rounded-b-apple-md" />
          </Show>
        </div>

        {/* 空调设备卡片 */}
        <div
          onClick={() => setActiveDevice(activeDevice() === 'ac' ? null : 'ac')}
          class={`
            relative cursor-pointer rounded-apple-md border transition-all duration-200
            ${deviceState().air_condition
              ? 'bg-apple-blue/10 border-apple-blue/30'
              : 'bg-dark-surface-3/50 border-white/[0.05] hover:border-white/[0.1]'}
          `}
        >
          <div class="p-4 flex flex-col items-center justify-center h-full gap-3">
            {/* 设备图标 */}
            <div class={`
              w-12 h-12 rounded-full flex items-center justify-center transition-colors
              ${deviceState().air_condition ? 'bg-apple-blue/20' : 'bg-dark-surface-4'}
            `}>
              <svg class={`w-6 h-6 ${deviceState().air_condition ? 'text-apple-blue' : 'text-apple-text-tertiary'}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
              </svg>
            </div>
            {/* 设备名称 */}
            <span class="text-xs font-medium text-apple-text-secondary">空调</span>
            {/* 状态指示 */}
            <span class={`text-[10px] ${deviceState().air_condition ? 'text-apple-blue' : 'text-apple-text-tertiary'}`}>
              {deviceState().air_condition ? `${deviceState().ac_temp}°C` : '关闭'}
            </span>
          </div>
          <Show when={deviceState().air_condition}>
            <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-apple-blue rounded-b-apple-md" />
          </Show>
        </div>
      </div>

      {/* 展开的设备控制面板 */}
      <Show when={activeDevice()}>
        <div class="mt-4 pt-4 border-t border-white/[0.05]">
          <Show when={activeDevice() === 'light'}>
            <Light />
          </Show>
          <Show when={activeDevice() === 'fan'}>
            <Fan />
          </Show>
          <Show when={activeDevice() === 'ac'}>
            <AirCondition />
          </Show>
        </div>
      </Show>
    </div>
  );
}
