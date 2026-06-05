import { Component } from 'solid-js';
import { LiquidGlass } from './LiquidGlass';

interface LiquidGlassSegmentToggleProps {
  leftLabel: string;
  rightLabel: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

export const LiquidGlassSegmentToggle: Component<LiquidGlassSegmentToggleProps> = (props) => {
  const handleClick = () => {
    props.onChange(!props.checked);
  };

  return (
    <button
      onClick={handleClick}
      class="relative h-9 min-w-[80px] cursor-pointer border-none p-0 active:scale-[0.97] transition-transform"
      style={{ background: 'transparent' }}
      role="group"
      aria-label={`${props.leftLabel} / ${props.rightLabel}`}
    >
      {/* 背景层：白色玻璃 */}
      <LiquidGlass
        radius={16}
        background="var(--color-glass-surface)"
        blur={8}
        edgeBlur={3}
        displacementScale={20}
        contrast={1.2}
        style={{ position: 'absolute', inset: '0', 'z-index': '2' }}
      />

      {/* 蓝色指示器 */}
      <div
        class="absolute top-0 left-0 w-1/2 h-full transition-all duration-200"
        style={{
          'z-index': '3',
          transform: props.checked ? 'translateX(100%)' : 'translateX(0)',
          'transition-timing-function': 'cubic-bezier(0.4, 0, 0.2, 1)',
        }}
      >
        <LiquidGlass
          radius={16}
          background="var(--color-glass-accent)"
          blur={6}
          edgeBlur={2}
          displacementScale={15}
          contrast={1.25}
          style={{ position: 'absolute', inset: '0', 'z-index': '2' }}
          class="w-full h-full"
        />
      </div>

      {/* 文字层（最顶层） */}
      <div class="absolute inset-0 flex pointer-events-none" style={{ 'z-index': '30' }}>
        <span
          class="flex-1 flex items-center justify-center text-xs font-medium"
          style={{ color: !props.checked ? 'var(--color-text-primary)' : 'var(--color-text-secondary)' }}
          role="radio"
          aria-checked={!props.checked}
        >
          {props.leftLabel}
        </span>
        <span
          class="flex-1 flex items-center justify-center text-xs font-medium"
          style={{ color: props.checked ? 'var(--color-text-primary)' : 'var(--color-text-secondary)' }}
          role="radio"
          aria-checked={props.checked}
        >
          {props.rightLabel}
        </span>
      </div>
    </button>
  );
};
