import { typedInvoke, getUserMessage } from '@/errors';
import { useShake } from '@/hooks/useShake';
import { RoomId, RoomDeviceState } from '../../store/deviceStore';
import { LiquidGlassToggle } from '../ui/LiquidGlassToggle';

interface AirConditionProps {
  roomId: RoomId;
  roomState: RoomDeviceState;
}

export function AirCondition(props: AirConditionProps) {
  const [shaking, triggerShake] = useShake();
  let toggling = false;
  const toggleAC = async () => {
    if (toggling) return;
    toggling = true;
    try {
      const newState = !props.roomState.air_condition.is_on;
      const result = await typedInvoke('handle_voice_command', {
        cmd: newState ? '开空调' : '关空调',
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

  const setTemp = async (value: number) => {
    const result = await typedInvoke('handle_voice_command', {
      cmd: `空调调到${value}度`,
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
          <span class="text-caption text-secondary">空调</span>
          <LiquidGlassToggle
            checked={props.roomState.air_condition.is_on}
            onChange={toggleAC}
          />
        </div>
      </div>

      {/* 温度调节 */}
      <div class="flex items-center gap-3">
        <button
          onClick={() => setTemp(Math.max(16, props.roomState.air_condition.temperature - 1))}
          class="control-button rounded-sm bg-black/[0.05] text-secondary hover:bg-black/[0.08] transition-all duration-200 flex items-center justify-center"
        >
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
          </svg>
        </button>
        <span class="text-lg font-light text-primary w-12 text-center">
          {props.roomState.air_condition.temperature}°
        </span>
        <button
          onClick={() => setTemp(Math.min(30, props.roomState.air_condition.temperature + 1))}
          class="control-button rounded-sm bg-black/[0.05] text-secondary hover:bg-black/[0.08] transition-all duration-200 flex items-center justify-center"
        >
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
        </button>
      </div>
    </div>
  );
}
