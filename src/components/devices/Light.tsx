import { Component } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { RoomId, RoomDeviceState } from '../../store/deviceStore';
import { LiquidGlassToggle } from '../ui/LiquidGlassToggle';

interface LightProps {
  roomId: RoomId;
  roomState: RoomDeviceState;
}

export const Light: Component<LightProps> = (props) => {
  const toggleLight = async () => {
    const newState = !props.roomState.light.is_on;
    try {
      await invoke('handle_voice_command', {
        cmd: newState ? '开灯' : '关灯',
        room: props.roomId,
      });
    } catch (e) {
      console.error('Failed to toggle light:', e);
    }
  };

  const setBrightness = async (value: number) => {
    try {
      await invoke('handle_voice_command', {
        cmd: `灯亮度调到${value}`,
        room: props.roomId,
      });
    } catch (e) {
      console.error('Failed to set brightness:', e);
    }
  };

  return (
    <div class="flex flex-col gap-3">
      <div class="flex items-center justify-between">
        {/* 开关 */}
        <div class="flex items-center gap-3">
          <span class="text-sm text-secondary">灯光</span>
          <LiquidGlassToggle
            checked={props.roomState.light.is_on}
            onChange={toggleLight}
          />
        </div>
      </div>

      {/* 亮度滑块 */}
      <div class="flex items-center gap-3">
        <span class="text-xs text-tertiary w-8">{props.roomState.light.brightness}%</span>
        <input
          type="range"
          min="0"
          max="100"
          value={props.roomState.light.brightness}
          onInput={(e) => setBrightness(parseInt(e.currentTarget.value))}
          class="flex-1 h-1 bg-black/[0.05] rounded-full appearance-none cursor-pointer
            [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4
            [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-accent [&::-webkit-slider-thumb]:shadow-md
            [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:transition-transform [&::-webkit-slider-thumb]:duration-150
            [&::-webkit-slider-thumb]:hover:scale-110"
        />
      </div>
    </div>
  );
};
