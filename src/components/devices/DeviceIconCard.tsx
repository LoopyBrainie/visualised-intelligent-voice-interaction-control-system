import { Component, createMemo } from 'solid-js';
import { deviceState, RoomId } from '../../store/deviceStore';
import { interactionContext } from '../../store/interactionStore';

type DeviceType = 'light' | 'fan' | 'ac' | 'curtain';

interface DeviceIconCardProps {
  type: DeviceType;
  roomId: RoomId;
  onClick: () => void;
  ref?: (el: HTMLButtonElement | undefined) => void;
}

const deviceLabels: Record<DeviceType, string> = {
  light: '灯光',
  fan: '风扇',
  ac: '空调',
  curtain: '窗帘',
};

const morandiOn: Record<DeviceType, string> = {
  light: 'var(--color-device-light-on)',
  fan: 'var(--color-device-fan-on)',
  ac: 'var(--color-device-ac-on)',
  curtain: 'var(--color-device-curtain-on)',
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

  const isGestureSelected = createMemo(() => {
    // 后端 selected_device 格式: "living_room.light" / "bedroom.ac" 等
    // 必须是 (roomId, type) 完全匹配 — 只让当前真正选中的那张卡亮蓝点,
    // 卧室灯 vs 客厅灯 不再同时被高亮
    const sel = interactionContext().selected_device;
    return sel === `${props.roomId}.${props.type}`;
  });

  const valueText = createMemo(() => {
    switch (props.type) {
      case 'light': return roomState().light.is_on ? `${roomState().light.brightness}%` : '关闭';
      case 'fan': return roomState().fan.is_on ? `档位 ${roomState().fan.speed}` : '关闭';
      case 'ac': return roomState().air_condition.is_on ? `${roomState().air_condition.temperature}°C` : '关闭';
      case 'curtain': return roomState().curtain.is_open ? '开启' : '关闭';
    }
  });

  const iconColor = () => isOn() ? morandiOn[props.type] : 'var(--color-device-off)';

  return (
    <button
      ref={(el) => { if (props.ref) props.ref(el); }}
      class="flex flex-col items-center gap-1 p-3 rounded-lg transition-all duration-200 cursor-pointer hover:bg-black/[0.03] active:scale-95"
      style={{ 'min-width': '0' }}
      onClick={props.onClick}
    >
      {/* Icon — scales with --icon-disc-size (40px / 48px / 56px by tier) */}
      <div
        class="relative flex items-center justify-center rounded-full transition-all duration-200"
        style={{
          width: 'var(--icon-disc-size, 48px)',
          height: 'var(--icon-disc-size, 48px)',
          border: isOn() || isGestureSelected()
            ? `2px solid ${isGestureSelected() ? 'var(--color-accent)' : 'rgba(0,113,227,0.3)'}`
            : '2px solid transparent',
          'background': isOn() ? `${morandiOn[props.type]}15` : 'transparent',
        }}
      >
        {props.type === 'light' && (
          <svg width="24" height="24" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="8" fill={iconColor()} opacity={isOn() ? 1 : 0.5} />
            <path d="M8 8 Q12 4 16 8 Q18 12 16 16 L8 16 Q6 12 8 8" fill={iconColor()} />
          </svg>
        )}
        {props.type === 'fan' && (
          <svg width="28" height="28" viewBox="0 0 32 32">
            <g style={{
              'transform-origin': '16px 16px',
              animation: isOn() ? 'fanSpin 1s ease-in-out infinite' : 'none',
            }}>
              <path
                d="M16 8 Q24 10 22 16 Q24 22 16 24 Q8 22 10 16 Q8 10 16 8"
                fill={iconColor()}
                opacity={isOn() ? 0.9 : 0.5}
              />
            </g>
            <circle cx="16" cy="16" r="3" fill={iconColor()} />
          </svg>
        )}
        {props.type === 'ac' && (
          <svg width="28" height="20" viewBox="0 0 36 24">
            <rect x="2" y="2" width="32" height="20" rx="4" fill={iconColor()} opacity={isOn() ? 1 : 0.5} />
            <rect x="6" y="6" width="24" height="2" rx="1" fill={isOn() ? '#94b8d1' : '#b8b6bc'} />
            <rect x="6" y="10" width="24" height="2" rx="1" fill={isOn() ? '#94b8d1' : '#b8b6bc'} />
            <rect x="6" y="14" width="24" height="2" rx="1" fill={isOn() ? '#94b8d1' : '#b8b6bc'} />
          </svg>
        )}
        {props.type === 'curtain' && (
          <svg width="24" height="28" viewBox="0 0 28 36">
            <rect x="2" y="2" width="24" height="32" rx="3" fill={iconColor()} opacity={isOn() ? 0.85 : 0.5} />
            <line x1="9" y1="4" x2="9" y2="32" stroke="rgba(0,0,0,0.12)" stroke-width="1" />
            <line x1="14" y1="4" x2="14" y2="32" stroke="rgba(0,0,0,0.12)" stroke-width="1" />
            <line x1="19" y1="4" x2="19" y2="32" stroke="rgba(0,0,0,0.12)" stroke-width="1" />
          </svg>
        )}

        {/* Gesture selection indicator */}
        {isGestureSelected() && (
          <div
            class="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full"
            style={{ background: 'var(--color-accent)' }}
          />
        )}
      </div>

      {/* Name — Caption */}
      <span
        class="text-caption text-center leading-tight truncate w-full"
        style={{ color: isOn() ? 'var(--color-text-primary)' : 'var(--color-text-tertiary)' }}
      >
        {deviceLabels[props.type]}
      </span>

      {/* State — Fine Print */}
      <span class="text-fine text-tertiary text-center">
        {valueText()}
      </span>
    </button>
  );
};
