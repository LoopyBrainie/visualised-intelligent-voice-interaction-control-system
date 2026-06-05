import { typedInvoke, getUserMessage } from '@/errors';
import { useShake } from '@/hooks/useShake';
import { RoomId, RoomDeviceState } from '../../store/deviceStore';
import { LiquidGlassToggle } from '../ui/LiquidGlassToggle';

interface FanProps {
  roomId: RoomId;
  roomState: RoomDeviceState;
}

export function Fan(props: FanProps) {
  const [shaking, triggerShake] = useShake();
  let toggling = false;
  const toggleFan = async () => {
    if (toggling) return;
    toggling = true;
    try {
      const newState = !props.roomState.fan.is_on;
      const result = await typedInvoke('handle_voice_command', {
        cmd: newState ? '开风扇' : '关风扇',
        room: props.roomId,
      });
      if (!result.ok) {
        console.error(getUserMessage(result.error));
        triggerShake();
      }
    } finally {
      toggling = false;
    }
  };

  const setSpeed = async (value: number) => {
    const result = await typedInvoke('handle_voice_command', {
      cmd: `风扇${value}档`,
      room: props.roomId,
    });
    if (!result.ok) {
      console.error(getUserMessage(result.error));
      triggerShake();
    }
  };

  return (
    <div class="flex flex-col gap-3" classList={{ 'animate-shake': shaking() }}>
      <div class="flex items-center justify-between">
        {/* 开关 */}
        <div class="flex items-center gap-3">
          <span class="text-caption text-secondary">风扇</span>
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
            class="control-button rounded-sm text-xs font-semibold transition-all duration-200"
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
