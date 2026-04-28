import { Component } from 'solid-js';
import { VisualizationPanel } from './components/VisualizationPanel';
import { SpectrumPanel } from './components/SpectrumPanel';
import { LogPanel } from './components/LogPanel';
import { ControlPanel } from './components/ControlPanel';

const App: Component = () => {
  return (
    <div class="min-h-screen bg-morandi-dark text-apple-text-primary font-sf-pro">
      {/* 顶部标题栏 (10%) */}
      <header class="h-[10vh] flex items-center justify-center border-b border-white/5">
        <h1 class="text-2xl font-semibold tracking-tight">
          可视化智能语音交互控制系统
        </h1>
      </header>

      {/* 主内容区 (80%) */}
      <main class="h-[80vh] flex">
        {/* 左侧：可视化仿真区 (60%) */}
        <section class="w-[60%] p-4">
          {/* 玻璃拟态卡片 */}
          <div class="h-full bg-dark-surface-1/80 backdrop-blur-glass rounded-apple-lg shadow-apple-card flex items-center justify-center">
            <VisualizationPanel />
          </div>
        </section>

        {/* 右侧：频谱区 + 日志区 (40%) */}
        <aside class="w-[40%] flex flex-col p-4 gap-4">
          {/* 音频频谱区 (30%) */}
          <div class="h-[30%] bg-dark-surface-1/80 backdrop-blur-glass rounded-apple-lg shadow-apple-card flex items-center justify-center">
            <SpectrumPanel />
          </div>

          {/* 实时日志区 (70%) */}
          <div class="h-[70%]">
            <LogPanel />
          </div>
        </aside>
      </main>

      {/* 底部：控制按钮区 (10%) */}
      <footer class="h-[10vh] flex items-center justify-center gap-4 border-t border-white/5">
        <ControlPanel />
      </footer>
    </div>
  );
};

export default App;
