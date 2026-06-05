import { JSX, ParentComponent, mergeProps, Show } from 'solid-js';

export interface LiquidGlassProps {
  radius?: number;
  background?: string;
  class?: string;
  style?: JSX.CSSProperties;
  ref?: (el: HTMLDivElement) => void;
  overlay?: boolean;
  /** Frost radius on the backdrop, in pixels. Drives `backdrop-filter: blur()`. */
  blur?: number;
  /**
   * @deprecated Kept for API compatibility. The current rim model derives its
   * visual properties solely from `displacementScale` via `box-shadow: inset`
   * geometry, so this prop is a no-op.
   */
  edgeBlur?: number;
  /**
   * Drives the rim thickness (`box-shadow: inset 0 0 0 Npx`) and the rim
   * highlight intensity. The geometry is aspect-ratio independent: every
   * edge gets exactly N pixels of rim, regardless of width/height.
   */
  displacementScale?: number;
  /** Contrast multiplier applied to the backdrop via CSS chain. */
  contrast?: number;
}

/**
 * Compute the rim shadow stack from `displacementScale`.
 *
 * The rim is three layered `box-shadow: inset` rings plus the legacy
 * top/bottom hairlines, all painted INSIDE the rounded-rect outline so they
 * automatically respect `border-radius`:
 *
 *   - Main rim thickness (`inset 0 0 0 Npx`)        — the glass body at the edge
 *   - 1px inner specular highlight                 — the reflective inner wall
 *   - 1px bottom dark line                         — the dense lower bezel
 *   - 1px top white hairline (replaces border-top) — the crisp top edge
 *   - 1px bottom dark hairline (replaces border-bottom) — the crisp bottom edge
 *
 * Because the inset shadows never cover the centre, the centre stays
 * naturally transparent — this IS the "centre transmission" of real glass.
 * Because the N is an absolute pixel count (not a percentage), the rim is
 * uniform on every side, regardless of width/height.
 */
function computeRimShadow(scale: number): string {
  // 1.5px (scale=10) → 3px (scale=40+). Capped so a giant scale doesn't
  // produce a 10px white band.
  const thickness = Math.max(1.5, Math.min(3, 1 + (scale - 10) / 30));
  // 0.20 (scale=10) → 0.50 (scale=60) rim alpha.
  const a = Math.max(0.20, Math.min(0.50, 0.20 + (scale - 10) / 100));
  const inner = a * 0.7; // inner specular highlight
  const bottom = 0.15; // bottom dark line

  return [
    `inset 0 0 0 ${thickness.toFixed(2)}px rgba(255, 255, 255, ${a.toFixed(2)})`,
    `inset 0 0 0 1px rgba(255, 255, 255, ${inner.toFixed(2)})`,
    `inset 0 -1px 0 rgba(0, 0, 0, ${bottom.toFixed(2)})`,
    `inset 0 1px 0 rgba(255, 255, 255, 0.85)`,
    `inset 0 -1px 0 rgba(0, 0, 0, 0.12)`,
  ].join(', ');
}

/**
 * LiquidGlass — pure-CSS liquid-glass material.
 *
 * Three visual effects, all in pure CSS so they run on the compositor thread
 * in WebView2 / Chromium without the SVG-filter compatibility pitfalls that
 * plague `feDisplacementMap` inside `backdrop-filter`:
 *
 *   1. Frost (Gaussian blur of backdrop)
 *      `backdrop-filter: blur(Npx) saturate(180%) brightness(1.05) contrast(K)`
 *      on a dedicated blur layer (z:-1) with `transform: scale(1.1)`. The
 *      scale gives the blur kernel room to sample outward, and the parent
 *      root's `overflow: hidden` cleanly crops the resulting edge smearing.
 *
 *   2. Edge refraction (lens-style rim highlight)
 *      Three layered `box-shadow: inset` rings on the root, tuned by
 *      `displacementScale`. Inset shadows paint INSIDE the element's
 *      rounded-rect outline, so they automatically respect `border-radius`
 *      and the element's shape — no SVG geometry required. The rim is
 *      aspect-ratio independent: every edge gets exactly N pixels of rim,
 *      regardless of width/height.
 *
 *   3. Centre transmission (clear middle, frosted edges)
 *      Achieved geometrically: `box-shadow: inset` does not cover the
 *      element's interior, so the centre is naturally transparent and the
 *      blur layer behind shows through. No mask is involved, so we sidestep
 *      WebView2/Chromium's mask-mode (alpha vs luminance) inconsistency.
 *
 * Compositing: the blur layer is a painting-only sibling (`z-index: -1`
 * inside an `isolation: isolate` root), and the content layer forces its
 * own compositor layer via `contain: paint` + `transform: translateZ(0)`.
 * All three layers' style objects are recomputed on prop change, but only
 * the affected CSS properties are touched — SolidJS does property-level
 * diff on `style={{}}` so this stays on the GPU compositor thread.
 */
export const LiquidGlass: ParentComponent<LiquidGlassProps> = (props) => {
  const merged = mergeProps(
    {
      radius: 16,
      background: 'rgba(235, 235, 240, 0.4)',
      overlay: true,
      blur: 20,
      edgeBlur: 6,
      displacementScale: 30,
      contrast: 1.15,
    },
    props
  );

  // CSS-side backdrop-filter chain. blur is the frost; saturate + brightness
  // recover the colour punch that blur removes; contrast is the final tone
  // curve. The whole list runs in one pass on the compositor thread.
  const backdropFilterCss = () =>
    `blur(${merged.blur}px) saturate(180%) brightness(1.05) contrast(${merged.contrast})`;

  const rootStyle = (): JSX.CSSProperties => ({
    position: 'relative',
    'border-radius': `${merged.radius}px`,
    isolation: 'isolate', // lock z:-1 blur inside root's stacking context
    'overflow': 'hidden', // crop the scale(1.1) blur overflow, kills edge smearing
    background: merged.background,
    'box-shadow': computeRimShadow(merged.displacementScale ?? 0),
    'will-change': 'backdrop-filter, transform',
    ...merged.style,
  });

  const blurStyle = (): JSX.CSSProperties => ({
    position: 'absolute',
    inset: '0',
    'border-radius': 'inherit',
    'transform': 'scale(1.1)',
    'transform-origin': 'center',
    'backdrop-filter': backdropFilterCss(),
    '-webkit-backdrop-filter': backdropFilterCss(),
    'pointer-events': 'none',
    'z-index': -1,
  });

  const highlightStyle = (): JSX.CSSProperties => ({
    position: 'absolute',
    inset: '0',
    'border-radius': 'inherit',
    'pointer-events': 'none',
    'z-index': 2,
    background: `
      radial-gradient(ellipse at 30% 15%, rgba(255,255,255,0.55) 0%, rgba(255,255,255,0.0) 55%),
      linear-gradient(135deg, rgba(255,255,255,0.30) 0%, rgba(255,255,255,0.0) 45%),
      linear-gradient(315deg, rgba(255,255,255,0.18) 0%, rgba(255,255,255,0.0) 30%)
    `,
    'box-shadow':
      'inset 0 1px 1px rgba(255,255,255,0.40), inset 0 -1px 1px rgba(0,0,0,0.06)',
  });

  const contentStyle = (): JSX.CSSProperties => ({
    position: 'relative',
    'z-index': 3,
    'contain': 'paint', // isolate content repaint from blur layer
    'transform': 'translateZ(0)', // force dedicated compositor layer, kill jaggies
  });

  return (
    <div
      ref={merged.ref}
      class={merged.class}
      data-liquid-glass=""
      style={rootStyle()}
    >
      {/* Blur layer — frosted backdrop with 10% outward oversampling. The
          outward portion (with edge smearing) is cropped by root's
          `overflow: hidden`, leaving only the fully-kernel-sampled inner
          90% visible. */}
      <div data-glass-blur style={blurStyle()} />

      {/* Highlight overlay (optional). Three gradient layers give a soft
          top-left specular highlight, a diagonal sweep, and a secondary
          bottom-right highlight — the "Apple top-light" cue. */}
      <Show when={merged.overlay}>
        <div data-glass-highlight style={highlightStyle()} />
      </Show>

      {/* Content — above the highlight and the backdrop-filter. The
          `contain: paint` + `transform: translateZ(0)` force a dedicated
          compositor layer, which keeps the rounded `border-radius` clean
          under WebView2's multi-layer hardware-accelerated compositing. */}
      <div data-glass-content style={contentStyle()}>
        {props.children}
      </div>
    </div>
  );
};
