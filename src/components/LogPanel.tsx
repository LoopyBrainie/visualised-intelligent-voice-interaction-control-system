import { createSignal, onMount, onCleanup, For } from 'solid-js';
import { listen } from '@tauri-apps/api/event';

interface LogEntry {
  timestamp: string;
  level: 'ERROR' | 'WARN' | 'INFO' | 'DEBUG';
  message: string;
}

const levelConfig: Record<string, { color: string; bg: string; label: string }> = {
  ERROR: { color: 'text-log-error', bg: 'bg-log-error/10', label: '错误' },
  WARN: { color: 'text-log-warn', bg: 'bg-log-warn/10', label: '警告' },
  INFO: { color: 'text-log-info', bg: 'bg-log-info/10', label: '信息' },
  DEBUG: { color: 'text-log-debug', bg: 'bg-log-debug/10', label: '调试' },
};

export function LogPanel() {
  const [logs, setLogs] = createSignal<LogEntry[]>([]);
  let scrollContainerRef: HTMLDivElement | undefined;
  let currentUnlisten: (() => void) | undefined;

  const scrollToBottom = () => {
    if (scrollContainerRef) {
      scrollContainerRef.scrollTop = scrollContainerRef.scrollHeight;
    }
  };

  onMount(async () => {
    // 预设测试日志
    const testLogs: LogEntry[] = [
      { timestamp: '00:00:00.000', level: 'INFO', message: '系统启动中...' },
      { timestamp: '00:00:00.100', level: 'DEBUG', message: '加载配置文件 ~/.config/app.json' },
      { timestamp: '00:00:00.200', level: 'INFO', message: '日志系统初始化完成' },
      { timestamp: '00:00:00.300', level: 'WARN', message: 'Python 环境未检测到，正在尝试初始化...' },
      { timestamp: '00:00:00.500', level: 'INFO', message: 'GUI 组件挂载完成' },
    ];
    setLogs(testLogs);

    // 监听 Rust 后端发送的日志事件 - 保持原有逻辑
    const unlistenFn = await listen<LogEntry>('log_event', (event) => {
      setLogs((prev) => [...prev, event.payload]);
      setTimeout(scrollToBottom, 10);
    });
    currentUnlisten = unlistenFn;
  });

  onCleanup(() => {
    if (currentUnlisten) {
      currentUnlisten();
    }
  });

  return (
    <div class="h-full flex flex-col bg-dark-surface-1/80 backdrop-blur-[30px] saturate-[140%] rounded-apple-md border border-white/[0.05]">
      {/* 标题栏 */}
      <div class="flex items-center justify-between px-4 py-3 border-b border-white/[0.05]">
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-apple-blue animate-pulse" />
          <h3 class="text-sm font-semibold text-apple-text-primary tracking-tight">
            实时日志
          </h3>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-xs text-apple-text-tertiary font-mono">
            {logs().length} 条
          </span>
          {/* 清空按钮 */}
          <button
            onClick={() => setLogs([])}
            class="text-xs text-apple-text-tertiary hover:text-apple-text-secondary transition-colors"
          >
            清空
          </button>
        </div>
      </div>

      {/* 日志列表 */}
      <div
        ref={scrollContainerRef}
        class="flex-1 overflow-y-auto p-3 space-y-1 font-mono text-xs"
      >
        <For each={logs()}>
          {(entry) => {
            const config = levelConfig[entry.level];
            return (
              <div class="flex items-start gap-3 py-1.5 px-3 rounded-apple-sm hover:bg-white/[0.02] transition-colors">
                {/* 时间戳 */}
                <span class="text-apple-text-tertiary shrink-0 w-24">
                  {entry.timestamp}
                </span>

                {/* 级别标签 */}
                <span class={`
                  px-2 py-0.5 rounded text-[10px] font-semibold uppercase shrink-0
                  ${config.color} ${config.bg}
                `}>
                  {config.label}
                </span>

                {/* 消息 */}
                <span class="text-apple-text-secondary flex-1 break-all leading-relaxed">
                  {entry.message}
                </span>
              </div>
            );
          }}
        </For>

        {/* 空状态 */}
        {logs().length === 0 && (
          <div class="flex flex-col items-center justify-center h-full text-apple-text-tertiary">
            <svg class="w-8 h-8 mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            <span class="text-xs">等待日志输入...</span>
          </div>
        )}
      </div>
    </div>
  );
}
