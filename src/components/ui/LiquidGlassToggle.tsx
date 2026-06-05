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
        background={props.checked ? 'var(--color-glass-toggle-on)' : 'var(--color-glass-toggle-off)'}
        blur={6}
        edgeBlur={2}
        displacementScale={15}
        contrast={1.2}
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
          background="var(--color-bg-secondary)"
          blur={4}
          edgeBlur={2}
          displacementScale={10}
          contrast={1.25}
          class="w-full h-full"
        />
      </div>
    </button>
  );
};
