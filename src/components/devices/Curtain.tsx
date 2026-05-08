import { invoke } from '@tauri-apps/api/core';
import { RoomId, RoomDeviceState } from '../../store/deviceStore';
import { LiquidGlassToggle } from '../ui/LiquidGlassToggle';

interface CurtainProps {
  roomId: RoomId;
  roomState: RoomDeviceState;
}

export function Curtain(props: CurtainProps) {
  const toggleCurtain = async () => {
    const newState = !props.roomState.curtain.is_open;
    try {
      await invoke('handle_voice_command', {
        cmd: newState ? '打开窗帘' : '关闭窗帘',
        room: props.roomId,
      });
    } catch (e) {
      console.error('Failed to toggle curtain:', e);
    }
  };

  return (
    <div class="flex items-center justify-between">
      <span class="text-sm text-secondary">窗帘</span>
      <LiquidGlassToggle
        checked={props.roomState.curtain.is_open}
        onChange={toggleCurtain}
      />
    </div>
  );
}
