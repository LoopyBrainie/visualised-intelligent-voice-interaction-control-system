import { Component, createMemo } from 'solid-js';
import { deviceState, RoomId } from '../../store/deviceStore';

type DeviceType = 'light' | 'fan' | 'ac' | 'curtain';

export type DeviceIconCardRef = HTMLButtonElement;

interface DeviceIconCardProps {
  type: DeviceType;
  roomId: RoomId;
  onClick: () => void;
  ref?: (el: HTMLButtonElement | undefined) => void;
}

// 设备图标颜色 — Morandi 装饰色（仅用于图标填充）
const deviceOnColors: Record<DeviceType, string> = {
  light: '#c9a86c',
  fan: '#8fae8b',
  ac: '#7da5c4',
  curtain: '#b8a9c9',
};

const deviceOffColors: Record<DeviceType, string> = {
  light: '#a8a49c',
  fan: '#a8a6a3',
  ac: '#a8a6ac',
  curtain: '#a8a4ac',
};

const deviceLabels: Record<DeviceType, string> = {
  light: '灯光',
  fan: '风扇',
  ac: '空调',
  curtain: '窗帘',
};

export const DeviceIconCard: Component<DeviceIconCardProps> = (props) => {
  const roomState = () => deviceState()[props.roomId];

  const isOn = createMemo(() => {
    switch (props.type) {
      case 'light': return roomState().light.is_on;
      case 'fan': return roomState().fan.is_on;
      case 'ac': return roomState().air_condition.is_on;
      case 'curtain': return roomState().curtain.is_open;
    }
  });

  const valueText = createMemo(() => {
    switch (props.type) {
      case 'light': return roomState().light.is_on ? `${roomState().light.brightness}%` : '';
      case 'fan': return roomState().fan.is_on ? `档位 ${roomState().fan.speed}` : '';
      case 'ac': return roomState().air_condition.is_on ? `${roomState().air_condition.temperature}°C` : '';
      case 'curtain': return roomState().curtain.is_open ? '开启' : '';
    }
  });

  const labelText = deviceLabels[props.type];
  const onColor = deviceOnColors[props.type];
  const offColor = deviceOffColors[props.type];

  return (
    <button
      ref={(el) => {
        if (props.ref) props.ref(el);
      }}
      class="device-card flex flex-col items-center justify-center gap-1 p-3 transition-all duration-200 cursor-pointer"
      style={{
        width: '80px',
        'min-width': '80px',
        background: isOn() ? `${onColor}99` : '#f5f5f7',
        'border-radius': 'var(--radius-md)',
        border: 'none',
        'box-shadow': isOn() ? '3px 5px 30px rgba(0, 0, 0, 0.22)' : 'none',
      }}
      onClick={props.onClick}
    >
      {/* 设备图标 */}
      <div class="relative">
        {props.type === 'light' && (
          <svg width="28" height="28" viewBox="0 0 24 24">
            <circle
              cx="12" cy="12" r="8"
              fill={isOn() ? onColor : offColor}
              opacity={isOn() ? 1 : 0.6}
              style={{
                filter: isOn() ? `drop-shadow(0 0 6px ${onColor})` : 'none',
              }}
            />
            <path
              d="M8 8 Q12 4 16 8 Q18 12 16 16 L8 16 Q6 12 8 8"
              fill={isOn() ? onColor : offColor}
            />
          </svg>
        )}
        {props.type === 'fan' && (
          <svg width="32" height="32" viewBox="0 0 32 32">
            <circle
              cx="16" cy="16" r="14"
              fill={isOn() ? onColor : offColor}
              opacity={isOn() ? 0.9 : 0.6}
            />
            <g style={{
              'transform-origin': '16px 16px',
              animation: isOn() ? 'fanSpin 1s linear infinite' : 'none',
            }}>
              <path
                d="M16 8 Q24 10 22 16 Q24 22 16 24 Q8 22 10 16 Q8 10 16 8"
                fill={isOn() ? '#9caf88' : offColor}
              />
            </g>
            <circle cx="16" cy="16" r="4" fill={isOn() ? onColor : offColor} />
          </svg>
        )}
        {props.type === 'ac' && (
          <svg width="36" height="24" viewBox="0 0 36 24">
            <rect
              x="2" y="2" width="32" height="20" rx="4"
              fill={isOn() ? onColor : offColor}
              opacity={isOn() ? 1 : 0.6}
            />
            <rect x="6" y="5" width="24" height="3" rx="1.5" fill={isOn() ? '#94b8d1' : '#b8b6bc'} />
            <rect x="6" y="10" width="24" height="3" rx="1.5" fill={isOn() ? '#94b8d1' : '#b8b6bc'} />
            <rect x="6" y="15" width="24" height="3" rx="1.5" fill={isOn() ? '#94b8d1' : '#b8b6bc'} />
          </svg>
        )}
        {props.type === 'curtain' && (
          <svg width="28" height="36" viewBox="0 0 28 36">
            <rect
              x="2" y="2" width="24" height="32" rx="3"
              fill={isOn() ? onColor : offColor}
              opacity={isOn() ? 0.85 : 0.5}
            />
            <line x1="9" y1="4" x2="9" y2="32" stroke="rgba(0,0,0,0.15)" stroke-width="1" />
            <line x1="14" y1="4" x2="14" y2="32" stroke="rgba(0,0,0,0.15)" stroke-width="1" />
            <line x1="19" y1="4" x2="19" y2="32" stroke="rgba(0,0,0,0.15)" stroke-width="1" />
          </svg>
        )}
      </div>

      {/* 设备名称 */}
      <span
        class="text-center truncate w-full text-sm"
        style={{
          color: isOn() ? onColor : 'rgba(0,0,0,0.48)',
        }}
      >
        {labelText}
      </span>

      {/* 状态值 */}
      <span
        class="text-center truncate w-full text-xs"
        style={{
          color: 'rgba(0,0,0,0.35)',
        }}
      >
        {valueText()}
      </span>
    </button>
  );
};
