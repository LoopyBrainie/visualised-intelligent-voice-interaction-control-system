import { Component } from 'solid-js';
import { VisualizationPanel } from './components/VisualizationPanel';
import { SpectrumPanel } from './components/SpectrumPanel';
import { LogPanel } from './components/LogPanel';
import { ControlPanel } from './components/ControlPanel';

const App: Component = () => {
  return (
    <div class="min-h-screen bg-morandi-dark text-apple-text-primary font-sf-pro">
      {/* 顶部标题栏 - Apple 风格紧凑导航 */}
      <header class="h-12 flex items-center justify-center border-b border-white/[0.05]">
        <h1 class="text-base font-semibold tracking-tight text-apple-text-primary">
          可视化智能语音交互控制系统
        </h1>
      </header>

      {/* 主内容区 - 80vh */}
      <main class="h-[calc(100vh-10rem)] flex gap-4 p-4">
        {/* 左侧：可视化仿真区 60% */}
        <section class="w-[60%] flex flex-col gap-4">
          {/* 设备卡片区域 */}
          <div class="flex-1 bg-dark-surface-1/80 backdrop-blur-[30px] saturate-[140%] rounded-apple-md border border-white/[0.05] shadow-apple-card p-4">
            <VisualizationPanel />
          </div>
        </section>

        {/* 右侧：频谱区 + 日志区 40% */}
        <aside class="w-[40%] flex flex-col gap-4">
          {/* 音频频谱区 35% */}
          <div class="h-[35%] bg-dark-surface-1/80 backdrop-blur-[30px] saturate-[140%] rounded-apple-md border border-white/[0.05] shadow-apple-card p-4">
            <SpectrumPanel />
          </div>

          {/* 实时日志区 65% */}
          <div class="h-[65%]">
            <LogPanel />
          </div>
        </aside>
      </main>

      {/* 底部：控制按钮区 */}
      <footer class="h-16 flex items-center justify-center gap-6 border-t border-white/[0.05]">
        <ControlPanel />
      </footer>
    </div>
  );
};

export default App;
