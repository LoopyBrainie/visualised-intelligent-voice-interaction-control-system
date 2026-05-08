import { JSX, ParentComponent, mergeProps, Show, ErrorBoundary } from 'solid-js'
import { useLiquidGlass, LiquidGlassOptions } from '../../hooks/useLiquidGlass'
import { useSnapshot } from './SnapshotContext'

interface LiquidGlassProps extends LiquidGlassOptions {
  radius?: number
  background?: string
  class?: string
  style?: JSX.CSSProperties
  ref?: (el: HTMLDivElement) => void
  overlay?: boolean
  /** Enable per-pixel displacement canvas. Disable for large containers where CSS backdrop-filter suffices. */
  displacement?: boolean
}

export const LiquidGlass: ParentComponent<LiquidGlassProps> = (props) => {
  const merged = mergeProps(
    {
      radius: 9999,
      blur: 0 as number | (() => number),
      contrast: 1.2,
      brightness: 1.05,
      saturate: 1.1,
      background: 'rgba(255, 255, 255, 0.6)',
      overlay: true,
      displacement: true,
    },
    props
  )

  const { canvas: snapshotCanvas, isReady, snapshotVersion } = useSnapshot()

  const {
    containerRef,
    displacedCanvasRef,
  } = useLiquidGlass({
    fragment: merged.fragment,
    canvasDPI: merged.canvasDPI,
    blur: merged.blur,
    snapshot: () => isReady() ? snapshotCanvas() : null,
    snapshotVersion,
  })

  const composedRef = (el: HTMLDivElement) => {
    containerRef(el)
    merged.ref?.(el)
  }

  const blurPx = () => typeof merged.blur === 'function' ? merged.blur() : merged.blur

  return (
    <ErrorBoundary
      fallback={(_err, _reset) => (
        <div
          ref={composedRef}
          class={merged.class}
          style={{
            'border-radius': `${merged.radius}px`,
            'backdrop-filter': `blur(${blurPx()}px) contrast(${merged.contrast}) brightness(${merged.brightness}) saturate(${merged.saturate})`,
            '-webkit-backdrop-filter': `blur(${blurPx()}px) contrast(${merged.contrast}) brightness(${merged.brightness}) saturate(${merged.saturate})`,
            background: merged.background,
            overflow: 'hidden',
            ...merged.style,
          }}
        >
          {props.children}
        </div>
      )}
    >
      <div
        ref={composedRef}
        class={merged.class}
        style={{
          position: 'relative',
          overflow: 'hidden',
          'border-radius': `${merged.radius}px`,
          background: merged.background,
          'backdrop-filter': `blur(${blurPx()}px) contrast(${merged.contrast}) brightness(${merged.brightness}) saturate(${merged.saturate})`,
          '-webkit-backdrop-filter': `blur(${blurPx()}px) contrast(${merged.contrast}) brightness(${merged.brightness}) saturate(${merged.saturate})`,
          '-webkit-font-smoothing': 'antialiased',
          '-moz-osx-font-smoothing': 'grayscale',
          'text-rendering': 'optimizeLegibility',
          ...merged.style,
        }}
      >
        {/* Displaced snapshot canvas — blur applied via two-canvas pipeline in hook */}
        <Show when={merged.displacement}>
          <canvas
            ref={displacedCanvasRef}
            data-glass-overlay
            style={{
              position: 'absolute',
              inset: '0',
              width: '100%',
              height: '100%',
              'border-radius': 'inherit',
              'pointer-events': 'none',
              'z-index': '0',
              'filter': `contrast(${merged.contrast}) brightness(${merged.brightness}) saturate(${merged.saturate})`,
              '-webkit-filter': `contrast(${merged.contrast}) brightness(${merged.brightness}) saturate(${merged.saturate})`,
            }}
          />
        </Show>
        {/* Content layer */}
        <div style={{ position: 'relative', 'z-index': '1' }}>
          {props.children}
        </div>
        <Show when={merged.overlay}>
          <div
            data-glass-overlay
            style={{
              position: 'absolute',
              inset: '0',
              'border-radius': 'inherit',
              'pointer-events': 'none',
              background: `
                radial-gradient(ellipse at 30% 20%, rgba(255,255,255,0.30) 0%, transparent 60%),
                linear-gradient(135deg, rgba(255,255,255,0.20) 0%, transparent 40%),
                linear-gradient(315deg, rgba(255,255,255,0.10) 0%, transparent 25%)
              `,
              'box-shadow': 'inset 0 1px 1px rgba(255,255,255,0.25), inset 0 -1px 1px rgba(0,0,0,0.05)',
              'mix-blend-mode': 'overlay',
            }}
          />
        </Show>
      </div>
    </ErrorBoundary>
  )
}
