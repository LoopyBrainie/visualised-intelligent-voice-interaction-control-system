import { invoke } from '@tauri-apps/api/core';
import { RoomId, RoomDeviceState } from '../../store/deviceStore';
import { LiquidGlassToggle } from '../ui/LiquidGlassToggle';

interface FanProps {
  roomId: RoomId;
  roomState: RoomDeviceState;
}

export function Fan(props: FanProps) {
  const toggleFan = async () => {
    const newState = !props.roomState.fan.is_on;
    try {
      await invoke('handle_voice_command', {
        cmd: newState ? '开风扇' : '关风扇',
        room: props.roomId,
      });
    } catch (e) {
      console.error('Failed to toggle fan:', e);
    }
  };

  const setSpeed = async (value: number) => {
    try {
      await invoke('handle_voice_command', {
        cmd: `风扇${value}档`,
        room: props.roomId,
      });
    } catch (e) {
      console.error('Failed to set fan speed:', e);
    }
  };

  return (
    <div class="flex flex-col gap-3">
      <div class="flex items-center justify-between">
        {/* 开关 */}
        <div class="flex items-center gap-3">
          <span class="text-sm text-secondary">风扇</span>
          <LiquidGlassToggle
            checked={props.roomState.fan.is_on}
            onChange={toggleFan}
          />
        </div>
      </div>

      {/* 速度档位 */}
      <div class="flex items-center gap-2">
        {[1, 2, 3].map((speed) => (
          <button
            onClick={() => setSpeed(speed)}
            class="w-8 h-8 rounded-sm text-xs font-semibold transition-all duration-200"
            style={{
              'background-color': props.roomState.fan.speed === speed
                ? 'var(--color-accent)'
                : 'rgba(0,0,0,0.05)',
              'color': props.roomState.fan.speed === speed
                ? 'white'
                : 'var(--color-text-secondary)'
            }}
          >
            {speed}
          </button>
        ))}
      </div>
    </div>
  );
}
