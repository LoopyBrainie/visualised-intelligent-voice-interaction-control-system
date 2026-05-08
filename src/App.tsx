import { Component, Show } from 'solid-js';
import { VisualizationPanel } from './components/VisualizationPanel';
import { SpectrumPanel } from './components/SpectrumPanel';
import { LogPanel } from './components/LogPanel';
import { ControlBar } from './components/ControlBar';
import { useResponsiveLayout } from './hooks/useResponsiveLayout';
import { SnapshotProvider } from './components/ui/SnapshotContext';

const App: Component = () => {
  const isPortrait = useResponsiveLayout();

  return (
    <SnapshotProvider>
    <div class="min-h-screen bg-primary text-primary font-sf-pro flex flex-col">
      {/* 顶部标题栏 — Apple 深色半透明导航玻璃 */}
      <header class="h-12 flex items-center justify-center shrink-0 z-nav glass-nav">
        <h1 class="text-micro text-white tracking-tight">
          可视化智能语音交互控制系统
        </h1>
      </header>

      {/* 主内容区 — 响应式布局 + 底部留空 */}
      <main class={`flex-1 min-h-0 flex ${isPortrait() ? 'flex-col' : 'flex-row'} gap-4 p-4 pb-24`}>
        {/* 左侧/顶部：可视化仿真区 */}
        <section class="flex flex-col gap-4 min-w-0 flex-1">
          <div class="flex-1 min-h-0 apple-card rounded-lg p-4 z-map">
            <VisualizationPanel />
          </div>
        </section>

        {/* 右侧/底部：频谱区 + 日志区 */}
        <Show when={!isPortrait()}>
          <aside class="w-80 h-[calc(100vh-8rem)] overflow-hidden flex flex-col gap-4 shrink-0 z-panel">
            <div class="h-[30%] min-h-[100px] apple-card rounded-lg p-4 shrink-0">
              <SpectrumPanel />
            </div>
            <div class="flex-1 min-h-0 apple-card rounded-lg overflow-hidden">
              <LogPanel />
            </div>
          </aside>
        </Show>

        <Show when={isPortrait()}>
          <aside class="w-full flex flex-col gap-4 shrink-0 z-panel">
            <div class="h-[100px] min-h-[80px] apple-card rounded-lg p-4 shrink-0">
              <SpectrumPanel />
            </div>
            <div class="flex-1 min-h-0 apple-card rounded-lg overflow-hidden">
              <LogPanel />
            </div>
          </aside>
        </Show>
      </main>

      {/* 底部：Apple 风格控制栏 */}
      <ControlBar isPortrait={isPortrait()} />
    </div>
    </SnapshotProvider>
  );
};

export default App;
