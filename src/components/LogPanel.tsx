import { createSignal, onMount, onCleanup, For } from 'solid-js';
import { listen } from '@tauri-apps/api/event';

interface LogEntry {
  timestamp: string;
  level: 'ERROR' | 'WARN' | 'INFO' | 'DEBUG';
  message: string;
}

const levelColors: Record<string, string> = {
  ERROR: 'text-log-error',
  WARN: 'text-log-warn',
  INFO: 'text-log-info',
  DEBUG: 'text-log-debug',
};

const levelBgColors: Record<string, string> = {
  ERROR: 'bg-log-error/10',
  WARN: 'bg-log-warn/10',
  INFO: 'bg-log-info/10',
  DEBUG: 'bg-log-debug/10',
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

    // 监听 Rust 后端发送的日志事件
    const unlistenFn = await listen<LogEntry>('log_event', (event) => {
      setLogs((prev) => [...prev, event.payload]);
      // 自动滚动到最新日志
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
    <div class="h-full flex flex-col bg-dark-surface-1/80 backdrop-blur-glass rounded-apple-md border border-white/5">
      {/* 标题栏 */}
      <div class="flex items-center justify-between px-4 py-3 border-b border-white/5">
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-apple-blue animate-pulse" />
          <h3 class="text-sm font-semibold text-apple-text-primary font-sf-pro">
            实时日志
          </h3>
        </div>
        <span class="text-xs text-apple-text-tertiary font-mono">
          {logs().length} 条
        </span>
      </div>

      {/* 日志列表 */}
      <div
        ref={scrollContainerRef}
        class="flex-1 overflow-y-auto p-3 space-y-1 font-mono text-xs"
      >
        <For each={logs()}>
          {(entry) => (
            <div class="flex items-start gap-3 py-1 px-2 rounded hover:bg-white/5 transition-colors">
              {/* 时间戳 */}
              <span class="text-apple-text-tertiary shrink-0">
                {entry.timestamp}
              </span>

              {/* 级别标签 */}
              <span
                class={`px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase shrink-0 ${levelColors[entry.level]} ${levelBgColors[entry.level]}`}
              >
                {entry.level}
              </span>

              {/* 消息 */}
              <span class="text-apple-text-secondary flex-1 break-all">
                {entry.message}
              </span>
            </div>
          )}
        </For>

        {/* 空状态 */}
        {logs().length === 0 && (
          <div class="flex items-center justify-center h-full text-apple-text-tertiary">
            <span>等待日志输入...</span>
          </div>
        )}
      </div>
    </div>
  );
}
