import { typedInvoke, getUserMessage } from '@/errors';
import { useShake } from '@/hooks/useShake';
import { RoomId, RoomDeviceState } from '../../store/deviceStore';
import { LiquidGlassToggle } from '../ui/LiquidGlassToggle';

interface CurtainProps {
  roomId: RoomId;
  roomState: RoomDeviceState;
}

export function Curtain(props: CurtainProps) {
  const [shaking, triggerShake] = useShake();
  let toggling = false;
  const toggleCurtain = async () => {
    if (toggling) return;
    toggling = true;
    try {
      const newState = !props.roomState.curtain.is_open;
      const result = await typedInvoke('handle_voice_command', {
        cmd: newState ? '打开窗帘' : '关闭窗帘',
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

  return (
    <div class="flex items-center justify-between" classList={{ 'animate-shake': shaking() }}>
      <span class="text-caption text-secondary">窗帘</span>
      <LiquidGlassToggle
        checked={props.roomState.curtain.is_open}
        onChange={toggleCurtain}
      />
    </div>
  );
}
