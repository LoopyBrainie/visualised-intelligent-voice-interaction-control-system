---
name: SPARV
description: A desktop smart home voice/gesture control system with Apple-inspired minimalism. Single accent color, SF Pro typography, LiquidGlass material as the visual signature.
colors:
  aperture-blue: "#0071e3"
  aperture-blue-hover: "#0077ed"
  aperture-blue-pressed: "#005bb4"
  ink: "#1d1d1f"
  canvas: "#ffffff"
  canvas-parchment: "#f5f5f7"
  surface-pearl: "#fafafc"
  surface-tile-dark: "#272729"
  surface-tile-dark-2: "#2a2a2c"
  surface-tile-dark-3: "#252527"
  surface-black: "#000000"
  on-primary: "#ffffff"
  on-dark: "#ffffff"
  body-muted: "#cccccc"
  ink-muted-80: "#333333"
  ink-muted-48: "#7a7a7a"
  hairline: "#e0e0e0"
  divider-soft: "#f0f0f0"
  chip-translucent: "#d2d2d7"
  log-error: "#ff3b30"
  log-warn: "#ff9500"
  log-info: "#0071e3"
  log-debug: "#8e8e93"
  device-light: "#d4a853"
  device-fan: "#7fb572"
  device-ac: "#6ba3c4"
  device-curtain: "#a89bc4"
  device-off: "#8e8e93"
typography:
  display-hero:
    fontFamily: "SF Pro Display, system-ui, -apple-system, sans-serif"
    fontSize: "56px"
    fontWeight: 600
    lineHeight: 1.07
    letterSpacing: "-0.28px"
  display-lg:
    fontFamily: "SF Pro Display, system-ui, -apple-system, sans-serif"
    fontSize: "40px"
    fontWeight: 600
    lineHeight: 1.1
    letterSpacing: "0"
  display-md:
    fontFamily: "SF Pro Text, system-ui, -apple-system, sans-serif"
    fontSize: "34px"
    fontWeight: 600
    lineHeight: 1.47
    letterSpacing: "-0.374px"
  lead:
    fontFamily: "SF Pro Display, system-ui, -apple-system, sans-serif"
    fontSize: "28px"
    fontWeight: 400
    lineHeight: 1.14
    letterSpacing: "0.196px"
  tagline:
    fontFamily: "SF Pro Text, system-ui, -apple-system, sans-serif"
    fontSize: "21px"
    fontWeight: 600
    lineHeight: 1.19
    letterSpacing: "0.231px"
  body-strong:
    fontFamily: "SF Pro Text, system-ui, -apple-system, sans-serif"
    fontSize: "17px"
    fontWeight: 600
    lineHeight: 1.24
    letterSpacing: "-0.374px"
  body:
    fontFamily: "SF Pro Text, system-ui, -apple-system, sans-serif"
    fontSize: "17px"
    fontWeight: 400
    lineHeight: 1.47
    letterSpacing: "-0.374px"
  caption:
    fontFamily: "SF Pro Text, system-ui, -apple-system, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.29
    letterSpacing: "-0.224px"
  caption-strong:
    fontFamily: "SF Pro Text, system-ui, -apple-system, sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.29
    letterSpacing: "-0.224px"
  fine-print:
    fontFamily: "SF Pro Text, system-ui, -apple-system, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.0
    letterSpacing: "-0.12px"
  mono:
    fontFamily: "ui-monospace, SF Mono, Menlo, Monaco, Consolas, monospace"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.0
    letterSpacing: "0"
rounded:
  xs: "5px"
  sm: "8px"
  md: "11px"
  lg: "18px"
  xl: "24px"
  pill: "9999px"
spacing:
  xxs: "4px"
  xs: "8px"
  sm: "12px"
  md: "17px"
  lg: "24px"
  xl: "32px"
  xxl: "48px"
  section: "80px"
components:
  button-primary:
    backgroundColor: "{colors.aperture-blue}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.pill}"
    padding: "11px 22px"
    typography: "{typography.body}"
  button-primary-active:
    backgroundColor: "{colors.aperture-blue-pressed}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.pill}"
    padding: "11px 22px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.aperture-blue}"
    rounded: "{rounded.pill}"
    padding: "11px 22px"
  button-pearl:
    backgroundColor: "{colors.surface-pearl}"
    textColor: "{colors.ink-muted-80}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-icon:
    backgroundColor: "{colors.chip-translucent}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
    size: "44px"
  card:
    backgroundColor: "{colors.canvas}"
    rounded: "{rounded.lg}"
    padding: "24px"
  toggle-track:
    backgroundColor: "{colors.chip-translucent}"
    rounded: "{rounded.pill}"
    size: "44x24px"
  toggle-thumb:
    backgroundColor: "{colors.canvas}"
    rounded: "{rounded.pill}"
    size: "20px"
---

# Design System: SPARV

## 1. Overview

**Creative North Star: "The Quiet Instrument"**

SPARV's interface behaves like a precision measurement tool: every pixel serves a function, every surface conveys state, and the UI itself disappears into the act of controlling devices. The user should never notice the design; they should only feel the fluency of speaking a command and watching their environment respond.

This system explicitly rejects the aesthetics PRODUCT.md names as anti-references: no cyberpunk neon, no dark-mode-with-purple-gradients, no industrial gray metal, no skeuomorphic buttons, no particle effects. The palette is deliberately restrained, the typography is system-native, and the single accent color (Aperture Blue) appears only where the user needs to act or observe state.

LiquidGlass, the SVG-filter-based refraction material, is SPARV's visual signature. It provides the only sense of depth in an otherwise flat system: the floating control bar and device popovers use it to separate interactive chrome from content. It is never decorative. If a surface doesn't need to float above content, it doesn't use LiquidGlass.

**Key Characteristics:**
- Single accent color (Aperture Blue #0071e3) used only for actions, selections, and state indicators
- SF Pro system font stack; no custom fonts, no display/body pairing
- Flat surfaces with glass layering for elevation; no shadows, no gradients on chrome
- Morandi-toned device icons provide subtle warmth without breaking the monochrome discipline
- 12px base unit spacing rhythm with 80px section gutters for visual breathing room

## 2. Colors

The palette is built on a single saturated accent surrounded by near-neutrals. Color communicates state, never decorates.

### Primary

- **Aperture Blue** (#0071e3): The only interactive color. Used for active toggles, selected states, spectrum bars, focus rings, and primary action buttons. Its role is functional: it marks "this is live" or "this is actionable." Never used as a background fill on non-interactive surfaces.

### Secondary

- **Device Morandi Tones**: Each device type retains a muted, Morandi-palette icon color for on-state identification. These are decorative at the icon level only and never appear on buttons, backgrounds, or text.
  - **Warm Gold** (#d4a853): Light device icon
  - **Sage Green** (#7fb572): Fan device icon
  - **Steel Blue** (#6ba3c4): Air conditioner icon
  - **Dusty Lavender** (#a89bc4): Curtain device icon
  - **Neutral Gray** (#8e8e93): All device off-states

### Neutral

- **Ink** (#1d1d1f): All text on light surfaces. Near-black, never pure black.
- **Canvas** (#ffffff): Primary surface for cards and content areas.
- **Parchment** (#f5f5f7): Page background, alternate tile sections. The subtle warmth prevents the sterile feel of pure white.
- **Pearl** (#fafafc): Secondary button fills, input backgrounds. One step warmer than canvas.
- **Hairline** (#e0e0e0): Card borders, dividers. Visible but never heavy.
- **Chip Translucent** (#d2d2d7): Circular control chips, media overlays at 64% opacity.
- **Muted 80** (#333333): Pearl button text, secondary emphasis.
- **Muted 48** (#7a7a7a): Disabled text, timestamps, metadata.
- **Body Muted** (#cccccc): Text on dark surfaces (secondary).

### Semantic

- **Error** (#ff3b30): Error logs, destructive actions. iOS system red.
- **Warning** (#ff9500): Warning logs, caution states. iOS system orange.
- **Info** (#0071e3): Informational logs. Reuses Aperture Blue.
- **Debug** (#8e8e93): Debug logs, lowest-priority metadata.

### Named Rules

**The Single Voice Rule.** Aperture Blue is the only saturated accent on screen. Device Morandi tones exist only inside icon SVGs and nowhere else. If you find yourself reaching for a second accent color, stop: the system needs one voice, not a chorus.

**The Never-Pure Rule.** No pure #000000 for text (use #1d1d1f Ink). No pure #ffffff for backgrounds behind text (use #f5f5f7 Parchment or #fafafc Pearl). Tinting neutrals prevents the harsh contrast that makes interfaces feel clinical.

## 3. Typography

**Display Font:** SF Pro Display (with system-ui, -apple-system, sans-serif fallback)
**Body Font:** SF Pro Text (with system-ui, -apple-system, sans-serif fallback)
**Mono Font:** ui-monospace, SF Mono, Menlo, Monaco, Consolas

**Character:** A single sans-serif family carries the entire interface. SF Pro's native rendering on macOS and Windows provides crisp CJK support without custom font loading. The hierarchy relies on weight contrast (400 vs 600) and size steps at a 1.2x ratio, not font-family swaps.

### Hierarchy

- **Hero** (600, 56px, 1.07, -0.28px): Reserved for splash screens and empty states. Negative letter-spacing tightens the large glyph.
- **Display LG** (600, 40px, 1.1): Section titles on full-width tiles.
- **Display MD** (600, 34px, 1.47, -0.374px): Area headers within tiles.
- **Lead** (400, 28px, 1.14, 0.196px): Subtitles and secondary headings on tiles. Slightly positive letter-spacing gives the lighter weight breathing room.
- **Tagline** (600, 21px, 1.19, 0.231px): Room labels, sub-navigation, section markers. Bold enough to anchor a section without dominating it.
- **Body Strong** (600, 17px, 1.24, -0.374px): Labels, device names, emphasized inline text.
- **Body** (400, 17px, 1.47, -0.374px): Default text. Max line length 65-75ch for prose; data-dense UI can run wider.
- **Caption** (400, 14px, 1.29, -0.224px): Metadata, timestamps, secondary labels.
- **Caption Strong** (600, 14px, 1.29, -0.224px): Emphasized metadata.
- **Fine Print** (400, 12px, 1.0, -0.12px): Footer text, trace IDs, lowest-priority information.
- **Mono** (400, 12px, 1.0): Log timestamps, technical identifiers. Monospaced for alignment.

### Named Rules

**The Weight Ladder Rule.** Only three weights exist: 300 (airy display text), 400 (body/caption), and 600 (headings/labels). Weight 500 is forbidden: it sits in an uncanny valley between regular and bold. Weight 700 appears in exactly one place (the legacy card-title class) and should be migrated to 600.

**The Negative-Space Rule.** Display sizes (40px+) use negative letter-spacing (-0.28px to -0.374px). Body sizes (17px and below) use negative or zero spacing. Only the Lead (28px) and Tagline (21px) use positive spacing, because their lighter weight needs the air.

## 4. Elevation

This system uses **glass layering** as its only elevation mechanism. There are no box-shadows on cards, buttons, or navigation. Depth is conveyed through three techniques:

1. **LiquidGlass material**: SVG-filter-based refraction with backdrop blur, used exclusively on floating interactive surfaces (ControlBar, device control popovers). The material itself separates the surface from its backdrop.
2. **Background tonal shifts**: Parchment (#f5f5f7) vs Canvas (#ffffff) vs Dark Tile (#272729) create visual separation through contrast, not shadow.
3. **Opacity layering**: Semi-transparent surfaces (rgba backgrounds at 60-80%) allow the underlying content to bleed through, creating a natural hierarchy.

Shadows in the codebase (such as `--shadow-subtle` and `--shadow-apple`) are legacy artifacts and should be removed. The only legitimate shadow is the drop-shadow under product imagery in marketing contexts, which this application does not have.

### Named Rules

**The Flat-By-Default Rule.** Every surface is flat at rest. If a surface needs to feel elevated, it gets LiquidGlass or a tonal background shift, never a shadow. If it looks like a 2014 app, the shadow is too dark and the blur is too small.

**The Glass-Only-for-Float Rule.** LiquidGlass is reserved for surfaces that genuinely float above content: the bottom control bar and device control popovers. It is forbidden on static cards, navigation bars, or section containers. Rarity is the point.

## 5. Components

### Buttons

- **Shape:** Pill radius (9999px) for primary and ghost buttons. Medium radius (11px) for pearl capsules. Full circle (9999px) for icon buttons.
- **Primary:** Aperture Blue (#0071e3) background, white text, 17px/600 weight, 11px 22px padding. Active state: scale(0.95) transform, no color change.
- **Ghost:** Transparent background, 1px Aperture Blue border, Aperture Blue text. Same padding and typography as primary.
- **Pearl Capsule:** Pearl (#fafafc) background, 3px soft divider (#f0f0f0) border, Muted 80 (#333333) text. Used for secondary actions like temperature controls.
- **Icon Circular:** Chip Translucent (#d2d2d7) at 64% opacity, Ink (#1d1d1f) icon, 44x44px fixed size. Used for floating media controls.

### Toggle (LiquidGlassToggle)

- **Track:** 44x24px, pill radius. Checked: Aperture Blue at 80% opacity. Unchecked: gray at 80% opacity. Both rendered through LiquidGlass SVG filter.
- **Thumb:** 20x20px white circle, pill radius. Slides between 2px and 22px translateX. LiquidGlass with minimal displacement (10px scale).
- **Transition:** 200ms ease on background color.

### Segment Toggle (LiquidGlassSegmentToggle)

- **Container:** 36px tall, 16px radius, white at 80% opacity background.
- **Indicator:** Sliding Aperture Blue at 60% opacity pill, 6px blur, 15px displacement. Animates with 200ms cubic-bezier(0.4, 0, 0.2, 1).
- **Labels:** 14px/400 weight, centered in each half.

### Cards

- **Corner Style:** Large radius (18px).
- **Background:** Canvas (#ffffff).
- **Border:** 1px solid hairline (#e0e0e0). No shadow.
- **Internal Padding:** 24px.
- **Content:** Device icon (centered), device name (Body Strong), status value (Caption, Muted 48).

### Device Cards (DeviceIconCard)

- **Size:** 88x112px fixed.
- **On-state:** Aperture Blue at 60% opacity background, white icon.
- **Off-state:** Parchment (#f5f5f7) background, Neutral Gray (#8e8e93) icon.
- **Gesture selected:** 2px solid Aperture Blue border (not a yellow dot).
- **Icon:** Inline SVG, 28x28px. Each device type has a unique path.

### Navigation Bar

- **Style:** Dark translucent glass. Background rgba(0,0,0,0.8), backdrop-filter saturate(180%) blur(20px). 48px tall.
- **Typography:** Fine Print (12px/400), white text, centered.
- **Position:** Fixed top, z-index 30.

### Control Bar (LiquidGlass)

- **Style:** LiquidGlass with heavy blur (20px), high displacement (40px), parchment 10% opacity background. 72px tall, fixed bottom.
- **Content:** MicButton (voice mode) or gesture status indicators (gesture mode), plus LiquidGlassSegmentToggle for mode switching.
- **Z-index:** 40 (above navigation).

### Floating Popover (LiquidGlass)

- **Style:** LiquidGlass with medium blur (15px), 35px displacement, white 80% opacity background. 16px radius.
- **Positioning:** solid-floating-ui with offset(10), flip(), shift() middleware.
- **Content:** Device-specific controls (toggle, brightness slider, speed buttons, temperature +/-).

### Inputs / Range Slider

- **Track:** 6px tall, pill radius, black 5% opacity background.
- **Thumb:** 20x20px, pill radius, Aperture Blue background with subtle blue shadow.
- **Focus:** Aperture Blue glow on thumb (0 2px 8px rgba(0,113,227,0.3)).

## 6. Do's and Don'ts

### Do:

- **Do** use Aperture Blue (#0071e3) as the single accent for all interactive states: toggles, selections, active indicators, spectrum bars, focus rings.
- **Do** use SF Pro system font stack for all text. No custom font loading; native rendering is faster and more reliable.
- **Do** use LiquidGlass for floating interactive surfaces (ControlBar, popovers) and nowhere else.
- **Do** use tonal background shifts (Parchment vs Canvas vs Dark Tile) to separate sections, not shadows.
- **Do** use Morandi device icon colors (#d4a853 gold, #7fb572 green, #6ba3c4 blue, #a89bc4 lavender) inside device icon SVGs only.
- **Do** keep all interactive targets at minimum 44x44px for touch accessibility.
- **Do** use 150-250ms transitions with cubic-bezier(0.4, 0, 0.2, 1) easing for state changes.
- **Do** use scale(0.95) on active press for buttons to provide tactile feedback.

### Don't:

- **Don't** use cyberpunk aesthetics: no neon colors, no dark themes as default, no glowing borders, no particle effects. (PRODUCT.md anti-reference: "游戏/赛博朋克风格")
- **Don't** use industrial control panel aesthetics: no gray metal textures, no skeuomorphic buttons, no outdated 3D effects. (PRODUCT.md anti-reference: "传统工业控制面板")
- **Don't** put shadows on cards, buttons, or navigation. Shadows are a legacy artifact; use glass layering or tonal shifts instead.
- **Don't** use pure #000000 for text or pure #ffffff for backgrounds behind text. Always tint toward the brand neutral.
- **Don't** use weight 500. It sits in an uncanny valley between 400 and 600.
- **Don't** use LiquidGlass on static surfaces (cards, sections, navigation). It is reserved for floating interactive chrome only.
- **Don't** use device Morandi colors on buttons, backgrounds, or text. They belong inside icon SVGs and nowhere else.
- **Don't** use decorative gradients on UI chrome. Gradients are forbidden on buttons, cards, navigation, and controls.
- **Don't** use display fonts (28px+) in UI labels, buttons, or data. Display sizes are for section headers only.
