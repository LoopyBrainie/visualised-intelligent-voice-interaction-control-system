import { createSignal, onCleanup } from 'solid-js';

/** Width tiers — mirrors the @media breakpoints in src/styles/app.css. */
export type WidthTier = 'n' | 's' | 'w';

/** Height tiers — 599px is the floor where gesture mode becomes unusable. */
export type HeightTier = 'h0' | 'h1' | 'h2';

/**
 * Visual density the layout should adopt. `compact` hides labels and shrinks
 * controls; `comfortable` uses the largest sizes; `standard` is the design
 * target. Derived from the (widthTier, heightTier) pair — see deriveDensity.
 */
export type Density = 'compact' | 'standard' | 'comfortable';

export interface ResponsiveLayout {
  width: number;
  height: number;
  widthTier: WidthTier;
  heightTier: HeightTier;
  density: Density;
}

const deriveDensity = (w: WidthTier, h: HeightTier): Density => {
  // h0 (height < 600) is always compact — gesture mode is forced to voice.
  if (h === 'h0') return 'compact';
  // Wide screens (and tall standard screens) earn the most breathing room.
  if (w === 'w') return 'comfortable';
  if (w === 'n') return 'compact';
  // s & h1 is the design target.
  // s & h2 (tall standard) gets more padding but stays at standard density.
  return 'standard';
};

export function useResponsiveLayout(): ResponsiveLayout {
  // Tier detection uses matchMedia — cheap, no per-pixel callback cost.
  // Initial values read from the live window so SSR / first-paint don't lie.
  const mqNarrow = typeof window !== 'undefined'
    ? window.matchMedia('(max-width: 1023px)')
    : null;
  const mqWide = typeof window !== 'undefined'
    ? window.matchMedia('(min-width: 1600px)')
    : null;
  const mqShort = typeof window !== 'undefined'
    ? window.matchMedia('(max-height: 599px)')
    : null;
  const mqTall = typeof window !== 'undefined'
    ? window.matchMedia('(min-height: 800px)')
    : null;

  const computeWidthTier = (): WidthTier => {
    if (mqNarrow?.matches) return 'n';
    if (mqWide?.matches) return 'w';
    return 's';
  };

  const computeHeightTier = (): HeightTier => {
    if (mqShort?.matches) return 'h0';
    if (mqTall?.matches) return 'h2';
    return 'h1';
  };

  const [widthTier, setWidthTier] = createSignal<WidthTier>(computeWidthTier());
  const [heightTier, setHeightTier] = createSignal<HeightTier>(computeHeightTier());
  const [size, setSize] = createSignal({ width: window.innerWidth, height: window.innerHeight });

  // Tier changes — matchMedia is event-driven, fires only on boundary crossings.
  const onWidthTierChange = () => setWidthTier(computeWidthTier());
  const onHeightTierChange = () => setHeightTier(computeHeightTier());
  mqNarrow?.addEventListener('change', onWidthTierChange);
  mqWide?.addEventListener('change', onWidthTierChange);
  mqShort?.addEventListener('change', onHeightTierChange);
  mqTall?.addEventListener('change', onHeightTierChange);

  // Continuous width/height — `resize` is the only source for exact numbers.
  const onResize = () => setSize({ width: window.innerWidth, height: window.innerHeight });
  window.addEventListener('resize', onResize);

  onCleanup(() => {
    mqNarrow?.removeEventListener('change', onWidthTierChange);
    mqWide?.removeEventListener('change', onWidthTierChange);
    mqShort?.removeEventListener('change', onHeightTierChange);
    mqTall?.removeEventListener('change', onHeightTierChange);
    window.removeEventListener('resize', onResize);
  });

  return {
    get width() { return size().width; },
    get height() { return size().height; },
    get widthTier() { return widthTier(); },
    get heightTier() { return heightTier(); },
    get density() { return deriveDensity(widthTier(), heightTier()); },
  };
}
