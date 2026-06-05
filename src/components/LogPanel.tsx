import { createSignal, onMount, onCleanup, For } from 'solid-js';
import { listen } from '@tauri-apps/api/event';

interface LogEntry {
  timestamp: string;
  level: 'ERROR' | 'WARN' | 'INFO' | 'DEBUG';
  message: string;
  trace_id?: string;
  source?: string;
}

const levelConfig: Record<string, { color: string; bg: string; border: string; label: string }> = {
  ERROR: { color: 'text-log-error', bg: 'bg-log-error/10', border: 'border-log-error/60', label: '错误' },
  WARN: { color: 'text-log-warn', bg: 'bg-log-warn/10', border: 'border-log-warn/60', label: '警告' },
  INFO: { color: 'text-log-info', bg: 'bg-log-info/10', border: 'border-log-info/50', label: '信息' },
  DEBUG: { color: 'text-log-debug', bg: 'bg-log-debug/10', border: 'border-log-debug/40', label: '调试' },
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
    // 预设测试日志（带 trace_id）
    const testLogs: LogEntry[] = [
      { timestamp: '00:00:00.000', level: 'INFO', message: '系统启动中...', source: 'rust' },
      { timestamp: '00:00:00.100', level: 'DEBUG', message: '加载配置文件 ~/.config/app.json', source: 'rust' },
      { timestamp: '00:00:00.200', level: 'INFO', message: '日志系统初始化完成', trace_id: 'a1b2c3d4', source: 'rust' },
      { timestamp: '00:00:00.300', level: 'WARN', message: 'Python 环境未检测到，正在尝试初始化...', trace_id: 'a1b2c3d4', source: 'daemon' },
      { timestamp: '00:00:00.500', level: 'INFO', message: 'GUI 组件挂载完成', source: 'rust' },
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
    <div class="h-full flex flex-col">
      {/* 标题栏 */}
      <div class="flex items-center justify-between px-4 py-3 border-b border-white/[0.05]">
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-accent animate-pulse" />
          <h3 class="text-sm font-semibold text-primary tracking-tight">
            实时日志
          </h3>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-xs text-tertiary font-mono">
            {logs().length} 条
          </span>
          {/* 清空按钮 */}
          <button
            onClick={() => setLogs([])}
            aria-label="清空日志"
            class="text-xs text-tertiary hover:text-secondary transition-colors"
          >
            清空
          </button>
        </div>
      </div>

      {/* 日志列表 */}
      <div
        ref={scrollContainerRef}
        class="flex-1 overflow-y-auto p-3 space-y-1 font-mono text-xs"
        role="log"
        aria-live="polite"
        aria-label="实时日志"
      >
        <For each={logs()}>
          {(entry) => {
            const config = levelConfig[entry.level];
            return (
              <div class="py-1.5 px-3 rounded-sm hover:bg-white/[0.02] transition-colors">
                {/* 元数据行：时间戳 + 级别 + 来源 + trace_id（flex-wrap 自动换行） */}
                <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                  {/* 时间戳 */}
                  <span class="text-tertiary shrink-0 w-24">
                    {entry.timestamp}
                  </span>

                  {/* 级别标签 */}
                  <span class={`
                    px-2 py-0.5 rounded text-[10px] font-semibold uppercase shrink-0
                    ${config.color} ${config.bg}
                  `}>
                    {config.label}
                  </span>

                  {/* 来源标签（如果有） */}
                  {entry.source && (
                    <span class="px-1.5 py-0.5 rounded text-[9px] shrink-0 bg-device-curtain-on/20 text-device-curtain-on">
                      {entry.source}
                    </span>
                  )}

                  {/* trace_id（如果有） */}
                  {entry.trace_id && (
                    <span class="px-1.5 py-0.5 rounded text-[9px] shrink-0 font-mono bg-device-ac-on/20 text-device-ac-on">
                      #{entry.trace_id}
                    </span>
                  )}
                </div>

                {/* 消息行：缩进对齐时间戳后，左侧 border 颜色跟随级别 */}
                <div class={`mt-1 ml-24 pl-2 border-l-2 ${config.border}`}>
                  <span class="text-secondary break-words leading-relaxed">
                    {entry.message}
                  </span>
                </div>
              </div>
            );
          }}
        </For>

        {/* 空状态 */}
        {logs().length === 0 && (
          <div class="flex flex-col items-center justify-center h-full text-tertiary">
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
