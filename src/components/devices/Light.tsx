import { Component, onCleanup } from 'solid-js';
import { typedInvoke, getUserMessage } from '@/errors';
import { useShake } from '@/hooks/useShake';
import { RoomId, RoomDeviceState } from '../../store/deviceStore';
import { LiquidGlassToggle } from '../ui/LiquidGlassToggle';

interface LightProps {
  roomId: RoomId;
  roomState: RoomDeviceState;
}

export const Light: Component<LightProps> = (props) => {
  const [shaking, triggerShake] = useShake();

  // 防止快速连击：写锁竞争 + 状态抖动
  let toggling = false;
  const toggleLight = async () => {
    if (toggling) return;
    toggling = true;
    try {
      const newState = !props.roomState.light.is_on;
      const result = await typedInvoke('handle_voice_command', {
        cmd: newState ? '开灯' : '关灯',
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

  // 拖动滑块会高频触发 onInput；debounce 150ms 让 Rust 只处理用户最终想停的位置
  let brightnessTimer: number | undefined;
  const setBrightness = (raw: number) => {
    // 防御性 clamp:即便外部传入越界值也不让命令超 0-100
    const value = Math.max(0, Math.min(100, Math.round(raw)));
    if (brightnessTimer !== undefined) clearTimeout(brightnessTimer);
    brightnessTimer = window.setTimeout(() => {
      typedInvoke('handle_voice_command', {
        cmd: `灯亮度调到${value}`,
        room: props.roomId,
      }).then((r) => {
        if (!r.ok) {
          console.error(getUserMessage(r.error));
          triggerShake();
        }
      });
    }, 150);
  };

  onCleanup(() => {
    if (brightnessTimer !== undefined) clearTimeout(brightnessTimer);
  });

  return (
    <div class="flex flex-col gap-3" classList={{ 'animate-shake': shaking() }}>
      <div class="flex items-center justify-between">
        {/* 开关 */}
        <div class="flex items-center gap-3">
          <span class="text-caption text-secondary">灯光</span>
          <LiquidGlassToggle
            checked={props.roomState.light.is_on}
            onChange={toggleLight}
          />
        </div>
      </div>

      {/* 亮度滑块 */}
      <div class="flex items-center gap-3">
        <span class="text-fine text-tertiary w-8">{props.roomState.light.brightness}%</span>
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
