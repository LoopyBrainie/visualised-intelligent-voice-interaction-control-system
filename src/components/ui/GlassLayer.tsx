import { JSX, ParentComponent, mergeProps, Show } from 'solid-js'

interface GlassLayerProps {
  radius?: number
  blur?: number
  background?: string
  overlay?: boolean
  class?: string
  style?: JSX.CSSProperties
  ref?: (el: HTMLDivElement) => void
}

export const GlassLayer: ParentComponent<GlassLayerProps> = (props) => {
  const merged = mergeProps(
    {
      radius: 9999,
      blur: 6,
      background: 'rgba(255, 255, 255, 0.4)',
      overlay: true,
    },
    props
  )

  return (
    <div
      ref={merged.ref}
      class={merged.class}
      style={{
        position: 'absolute',
        inset: '0',
        'z-index': '1',
        'border-radius': `${merged.radius}px`,
        'backdrop-filter': `blur(${merged.blur}px) saturate(1.2)`,
        '-webkit-backdrop-filter': `blur(${merged.blur}px) saturate(1.2)`,
        background: merged.background,
        overflow: 'hidden',
        ...merged.style,
      }}
    >
      {merged.children}
      <Show when={merged.overlay}>
        <div
          style={{
            position: 'absolute',
            inset: '0',
            'border-radius': 'inherit',
            'pointer-events': 'none',
            background: `
              radial-gradient(ellipse at 30% 20%, rgba(255,255,255,0.20) 0%, transparent 60%),
              linear-gradient(135deg, rgba(255,255,255,0.12) 0%, transparent 40%)
            `,
            'box-shadow': 'inset 0 1px 1px rgba(255,255,255,0.20), inset 0 -1px 1px rgba(0,0,0,0.04)',
            'mix-blend-mode': 'soft-light',
          }}
        />
      </Show>
    </div>
  )
}
