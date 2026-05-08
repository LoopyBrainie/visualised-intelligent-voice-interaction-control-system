import { Component } from 'solid-js';
import { LiquidGlass } from './LiquidGlass';

interface LiquidGlassToggleProps {
  checked: boolean;
  onChange: () => void;
  disabled?: boolean;
}

export const LiquidGlassToggle: Component<LiquidGlassToggleProps> = (props) => {
  return (
    <button
      onClick={props.onChange}
      disabled={props.disabled}
      class="relative w-11 h-6 cursor-pointer border-none p-0"
      style={{ background: 'transparent', overflow: 'hidden' }}
    >
      {/* 轨道 */}
      <LiquidGlass
        radius={9999}
        blur={8}
        contrast={1.3}
        brightness={1.06}
        background={props.checked ? 'rgba(0, 113, 227, 0.8)' : 'rgba(138, 138, 138, 0.8)'}
        style={{ position: 'absolute', inset: '0', transition: 'background 200ms ease' }}
      />
      {/* 滑块 */}
      <div
        class="absolute top-0.5 w-5 h-5 transition-transform duration-200"
        style={{
          left: '0px',
          'z-index': '3',
          transform: props.checked ? 'translateX(22px)' : 'translateX(2px)',
        }}
      >
        <LiquidGlass
          radius={9999}
          blur={8}
          contrast={1.3}
          brightness={1.06}
          background="rgba(255, 255, 255, 1)"
          class="w-full h-full"
        />
      </div>
    </button>
  );
};
