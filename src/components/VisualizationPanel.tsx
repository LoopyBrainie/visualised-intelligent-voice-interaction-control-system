import { Component, createSignal, Show } from 'solid-js';
import { Light } from './devices/Light';
import { Fan } from './devices/Fan';
import { AirCondition } from './devices/AirCondition';
import { deviceState } from '../store/deviceStore';

// Device color tokens (matching tailwind.config.js design tokens)
const deviceColors = {
  light: {
    on: '#fbbf24',    // device-light
    off: '#4b5563',
    glow: 'rgba(251, 191, 36,',  // For drop-shadow
    label: '#9ca3af',
  },
  fan: {
    on: '#34d399',    // device-fan
    off: '#4b5563',
    blade: '#10b981',
    bladeAlt: '#059669',
    center: '#10b981',
    label: '#9ca3af',
  },
  ac: {
    on: '#60a5fa',    // device-ac
    onAlt: '#93c5fd',
    off: '#4b5563',
    offAlt: '#6b7280',
    label: '#9ca3af',
    value: '#6b7280',
  },
  curtain: {
    on: '#a78bfa',    // device-curtain
    off: '#4b5563',
    label: '#9ca3af',
  },
};

export function VisualizationPanel() {
  const [activeDevice, setActiveDevice] = createSignal<'light' | 'fan' | 'ac' | null>(null);

  // 计算发光透明度：线性映射 0.5 + (brightness / 200)，范围 [0.5, 1.0]
  const getGlowOpacity = (brightness: number) => 0.5 + (brightness / 200);

  return (
    <div class="h-full flex flex-col">
      {/* 设备选择器标签 */}
      <div class="flex items-center gap-2 mb-4">
        <div class="w-2 h-2 rounded-full bg-apple-blue animate-pulse" />
        <h3 class="text-sm font-semibold text-apple-text-primary tracking-tight">
          设备控制中心
        </h3>
      </div>

      {/* SVG 房间平面图 - 2D矢量布局 */}
      <div class="flex-1 relative">
        <svg
          viewBox="0 0 400 280"
          class="w-full h-full"
          style={{ "max-height": "100%" }}
        >
          {/* 背景 - 低饱和度灰调 */}
          <rect x="0" y="0" width="400" height="280" fill="#2a2d35" rx="8" />

          {/* 客厅区域 - 左半部分 */}
          <g id="living-room">
            <rect x="10" y="10" width="240" height="180" fill="#32363e" stroke="#3d424a" stroke-width="1" rx="4" />
            <text x="20" y="30" fill="#6b7280" font-size="10" font-family="system-ui">客厅</text>
          </g>

          {/* 卧室区域 - 右半部分 */}
          <g id="bedroom">
            <rect x="260" y="10" width="130" height="180" fill="#32363e" stroke="#3d424a" stroke-width="1" rx="4" />
            <text x="270" y="30" fill="#6b7280" font-size="10" font-family="system-ui">卧室</text>
          </g>

          {/* 灯光设备 - 客厅左上角 */}
          <g
            id="light-device"
            transform="translate(50, 60)"
            onClick={() => setActiveDevice(activeDevice() === 'light' ? null : 'light')}
            style={{ cursor: 'pointer' }}
          >
            {/* 发光效果容器 - will-change优化 */}
            <g class="will-change-filter" style={{
              filter: deviceState().light && deviceState().light_brightness > 50
                ? `drop-shadow(0 0 ${8 + deviceState().light_brightness / 10}px ${deviceColors.light.glow}${getGlowOpacity(deviceState().light_brightness)}))`
                : 'none'
            }}>
              {/* 灯泡图标 */}
              <circle
                cx="0" cy="0" r="18"
                fill={deviceState().light ? deviceColors.light.on : deviceColors.light.off}
                opacity={deviceState().light ? 1 : 0.5}
              />
              <path
                d="M-6 -6 Q0 -14 6 -6 Q10 2 6 8 L-6 8 Q-10 2 -6 -6"
                fill={deviceState().light ? deviceColors.light.on : deviceColors.light.off}
                stroke={deviceState().light ? '#f59e0b' : deviceColors.light.off}
                stroke-width="1"
              />
            </g>
            {/* 设备标签 */}
            <text
              x="0" y="35"
              text-anchor="middle"
              fill={deviceState().light ? deviceColors.light.on : deviceColors.light.label}
              font-size="11"
              font-family="system-ui"
            >
              灯光
            </text>
            {/* 亮度值 */}
            <text
              x="0" y="48"
              text-anchor="middle"
              fill="#6b7280"
              font-size="9"
              font-family="system-ui"
            >
              {deviceState().light ? `${deviceState().light_brightness}%` : '关闭'}
            </text>
          </g>

          {/* 空调设备 - 客厅右上角 */}
          <g
            id="ac-device"
            transform="translate(180, 60)"
            onClick={() => setActiveDevice(activeDevice() === 'ac' ? null : 'ac')}
            style={{ cursor: 'pointer' }}
          >
            {/* 空调图标 */}
            <rect
              x="-22" y="-14" width="44" height="28" rx="4"
              fill={deviceState().air_condition ? deviceColors.ac.on : deviceColors.ac.off}
              opacity={deviceState().air_condition ? 1 : 0.5}
            />
            <rect x="-18" y="-10" width="36" height="4" rx="2" fill={deviceState().air_condition ? deviceColors.ac.onAlt : deviceColors.ac.offAlt} />
            <rect x="-18" y="-2" width="36" height="4" rx="2" fill={deviceState().air_condition ? deviceColors.ac.onAlt : deviceColors.ac.offAlt} />
            <rect x="-18" y="6" width="36" height="4" rx="2" fill={deviceState().air_condition ? deviceColors.ac.onAlt : deviceColors.ac.offAlt} />
            {/* 设备标签 */}
            <text
              x="0" y="28"
              text-anchor="middle"
              fill={deviceState().air_condition ? deviceColors.ac.on : deviceColors.ac.label}
              font-size="11"
              font-family="system-ui"
            >
              空调
            </text>
            {/* 温度值 */}
            <text
              x="0" y="42"
              text-anchor="middle"
              fill={deviceState().air_condition ? deviceColors.ac.on : deviceColors.ac.value}
              font-size="10"
              font-family="system-ui"
              font-weight="500"
            >
              {deviceState().air_condition ? `${deviceState().ac_temp}°C` : '关闭'}
            </text>
          </g>

          {/* 风扇设备 - 客厅下方 */}
          <g
            id="fan-device"
            transform="translate(115, 140)"
            onClick={() => setActiveDevice(activeDevice() === 'fan' ? null : 'fan')}
            style={{ cursor: 'pointer' }}
          >
            {/* 风扇主体 */}
            <circle
              cx="0" cy="0" r="22"
              fill={deviceState().fan ? deviceColors.fan.on : deviceColors.fan.off}
              opacity={deviceState().fan ? 0.9 : 0.5}
            />
            {/* 风扇叶片 - 旋转动画 */}
            <g style={{
              transform: deviceState().fan ? `rotate(${Date.now() / 10 % 360}deg)` : 'none',
              'transform-origin': '0 0',
              transition: 'transform 0.1s linear'
            }}>
              <path d="M0 -8 Q12 -12 8 0 Q12 12 0 8 Q-12 12 -8 0 Q-12 -12 0 -8" fill={deviceState().fan ? deviceColors.fan.blade : deviceColors.fan.off} />
              <path d="M-8 0 Q-12 12 0 8 Q12 12 8 0 Q12 -12 0 -8 Q-12 -12 -8 0" fill={deviceState().fan ? deviceColors.fan.bladeAlt : deviceColors.fan.off} />
            </g>
            <circle cx="0" cy="0" r="4" fill={deviceState().fan ? deviceColors.fan.center : deviceColors.fan.off} />
            {/* 设备标签 */}
            <text
              x="0" y="38"
              text-anchor="middle"
              fill={deviceState().fan ? deviceColors.fan.on : deviceColors.fan.label}
              font-size="11"
              font-family="system-ui"
            >
              风扇
            </text>
            {/* 风速值 */}
            <text
              x="0" y="52"
              text-anchor="middle"
              fill="#6b7280"
              font-size="9"
              font-family="system-ui"
            >
              {deviceState().fan ? `档位 ${deviceState().fan_speed}` : '关闭'}
            </text>
          </g>

          {/* 窗帘设备 - 卧室 */}
          <g
            id="curtain-device"
            transform="translate(325, 100)"
          >
            <rect
              x="-18" y="-25" width="36" height="50" rx="3"
              fill={deviceState().curtain ? deviceColors.curtain.on : deviceColors.curtain.off}
              opacity={deviceState().curtain ? 0.8 : 0.4}
            />
            <line x1="-12" y1="-20" x2="-12" y2="20" stroke="#6b7280" stroke-width="1" />
            <line x1="0" y1="-20" x2="0" y2="20" stroke="#6b7280" stroke-width="1" />
            <line x1="12" y1="-20" x2="12" y2="20" stroke="#6b7280" stroke-width="1" />
            <text
              x="0" y="40"
              text-anchor="middle"
              fill={deviceState().curtain ? deviceColors.curtain.on : deviceColors.curtain.label}
              font-size="10"
              font-family="system-ui"
            >
              窗帘
            </text>
          </g>

          {/* 房间门标识 */}
          <rect x="185" y="190" width="30" height="8" fill="#3d424a" rx="2" />
          <text x="200" y="215" text-anchor="middle" fill="#4b5563" font-size="8" font-family="system-ui">入口</text>
        </svg>
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
