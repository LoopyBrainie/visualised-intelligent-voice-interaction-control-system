import { Component, createEffect, createSignal } from 'solid-js';
import { VisualizationPanel } from './components/VisualizationPanel';
import { SpectrumPanel } from './components/SpectrumPanel';
import { ControlBar } from './components/ControlBar';
import { useResponsiveLayout } from './hooks/useResponsiveLayout';

const App: Component = () => {
  const [currentMode, setCurrentMode] = createSignal<'voice' | 'gesture'>('voice');
  const [viewMode, setViewMode] = createSignal<'devices' | 'diagnostics'>('devices');

  const layout = useResponsiveLayout();

  // At very short windows (h0, < 600px tall) the gesture-mode 360px camera
  // strip crowds the main content. Force-collapse to voice so the user
  // keeps a usable surface; they can switch back manually if they resize up.
  createEffect(() => {
    if (layout.heightTier === 'h0' && currentMode() === 'gesture') {
      setCurrentMode('voice');
    }
  });

  return (
    <div class={`h-screen flex flex-col overflow-hidden bg-primary text-primary ${currentMode() === 'gesture' ? 'gesture-mode' : ''}`}>
      {/* Header — 36px, Parchment, minimal */}
      <header
        class="shrink-0 flex items-center justify-center border-b border-hairline z-nav"
        style={{ height: 'var(--header-height)' }}
      >
        <h1 class="text-fine text-secondary tracking-wide uppercase">
          SPARV
        </h1>
      </header>

      {/* Media Strip — 140px, fixed, ambient morphing */}
      <div
        class="shrink-0 bg-secondary z-media flex items-center justify-center"
        style={{ height: 'var(--media-strip-height)' }}
      >
        <SpectrumPanel mode={currentMode()} />
      </div>

      {/* Main — stacked rooms, scrollable. Top padding is generous to
          separate the room grid from the Canvas media strip; the
          Parchment-to-Canvas-to-Parchment tone shift does the rest. */}
      <main
        class="flex-1 min-h-0 overflow-y-auto"
        style={{
          'padding-top': 'var(--content-padding-y)',
          'padding-left': 'var(--content-padding-x)',
          'padding-right': 'var(--content-padding-x)',
          'padding-bottom': 'calc(var(--control-bar-height) + var(--content-padding-y))',
        }}
      >
        <VisualizationPanel />
      </main>

      {/* ControlBar — 72px, LiquidGlass floating */}
      <ControlBar
        mode={currentMode()}
        viewMode={viewMode()}
        onModeChange={setCurrentMode}
        onViewChange={setViewMode}
      />
    </div>
  );
};

export default App;
