import { createSignal, onCleanup, onMount, Show, For } from 'solid-js';
import { Portal } from 'solid-js/web';
import { listen } from '@tauri-apps/api/event';
import { typedInvoke, getUserMessage } from '@/errors';
import { LiquidGlassSegmentToggle } from './ui/LiquidGlassSegmentToggle';
import { MicButton } from './ui/MicButton';
import { LiquidGlass } from './ui/LiquidGlass';
import { interactionContext, resetInteractionContext, type GestureHealth } from '../store/interactionStore';
import { useResponsiveLayout } from '../hooks/useResponsiveLayout';

interface ControlBarProps {
  mode: 'voice' | 'gesture';
  viewMode: 'devices' | 'diagnostics';
  onModeChange?: (mode: 'voice' | 'gesture') => void;
  onViewChange?: (view: 'devices' | 'diagnostics') => void;
}

interface CameraInfo {
  id: string;
  name: string;
  description: string;
}

// 与 python_engine/gesture.py 输出的 count 数值(0..5)一一对应
const gestureLabels: Record<number, string> = {
  0: '✊ 取消',
  1: '☝️ 选中',
  2: '✌️ 开关',
  3: '🤟 全开',
  4: '🖖 全关',
  5: '🖐️ 全部切换',
};

const CAMERA_STORAGE_KEY = 'selected-camera-id';

export function ControlBar(props: ControlBarProps) {
  const layout = useResponsiveLayout();
  const [cameras, setCameras] = createSignal<CameraInfo[]>([]);
  const [selectedCameraId, setSelectedCameraId] = createSignal<string | null>(
    localStorage.getItem(CAMERA_STORAGE_KEY)
  );
  const [showCameraMenu, setShowCameraMenu] = createSignal(false);
  const [cameraMenuPos, setCameraMenuPos] = createSignal<{ bottom: string; left: string }>({ bottom: '0px', left: '0px' });
  const [gestureError, setGestureError] = createSignal<string | null>(null);
  let cameraBtnRef: HTMLButtonElement | undefined;
  let gestureErrorTimer: number | undefined;

  const toggleCameraMenu = () => {
    if (!showCameraMenu() && cameraBtnRef) {
      const rect = cameraBtnRef.getBoundingClientRect();
      setCameraMenuPos({
        bottom: `${layout.height - rect.top + 8}px`,
        left: `${Math.max(8, rect.left)}px`,
      });
    }
    setShowCameraMenu(v => !v);
  };

  const closeMenu = (e: MouseEvent) => {
    const target = e.target as HTMLElement;
    if (cameraBtnRef && !cameraBtnRef.contains(target) && !target.closest('[data-camera-menu]')) {
      setShowCameraMenu(false);
    }
  };

  // 暴露启动链路的真实错误到 UI(修复 A:避免 invoke 错误被 console.error 静默吞掉)
  // 5 秒后自动消失,但用户可点击 ✕ 立即关闭
  const showGestureError = (message: string) => {
    setGestureError(message);
    if (gestureErrorTimer !== undefined) clearTimeout(gestureErrorTimer);
    gestureErrorTimer = window.setTimeout(() => setGestureError(null), 5000);
  };

  onMount(() => {
    typedInvoke('set_language_mode', { enabled: props.mode === 'voice' }).then(r => {
      if (!r.ok) console.error(getUserMessage(r.error));
    });

    typedInvoke<CameraInfo[]>('list_cameras').then(r => {
      if (!r.ok) { console.error(getUserMessage(r.error)); return; }
      setCameras(r.value);
    });

    document.addEventListener('click', closeMenu);

    // 修复 A+: 订阅 Rust 端 log_event，过滤 [Gesture] ERROR 级别自动填入横幅
    // 关键: start_gesture_capture 即便摄像头初始化失败也返回 Ok()，
    // 真实错误在 lib.rs:466-493 路径中只能通过 log_event / gesture-health-changed 拿到。
    // 同时订阅 health 事件以便把"摄像头离线/Python 离线"用横幅强化。
    const unlistenLog = listen<{ level: string; message: string }>('log_event', (event) => {
      const { level, message } = event.payload;
      if (level === 'ERROR' && /\[Gesture\]/.test(message)) {
        showGestureError(message);
      }
    });

    const unlistenHealth = listen<GestureHealth>('gesture-health-changed', (event) => {
      const h = event.payload;
      if (!h.camera_ok) {
        showGestureError(`摄像头离线: ${h.camera_name ?? '未识别设备'}`);
      } else if (!h.python_ok) {
        showGestureError('Python 守护进程离线');
      }
    });

    onCleanup(() => {
      document.removeEventListener('click', closeMenu);
      if (gestureErrorTimer !== undefined) clearTimeout(gestureErrorTimer);
      unlistenLog.then(fn => fn());
      unlistenHealth.then(fn => fn());
    });
  });

  const selectedCamera = () => cameras().find(c => c.id === selectedCameraId()) ?? null;

  const selectCamera = async (cameraId: string) => {
    setShowCameraMenu(false);
    const resolvedId = cameraId || null;
    if (resolvedId === selectedCameraId()) return;

    setSelectedCameraId(resolvedId);
    if (resolvedId) {
      localStorage.setItem(CAMERA_STORAGE_KEY, resolvedId);
    } else {
      localStorage.removeItem(CAMERA_STORAGE_KEY);
    }

    if (props.mode === 'gesture') {
      const r1 = await typedInvoke('stop_gesture_capture');
      if (!r1.ok) console.error(getUserMessage(r1.error));
      const r2 = await typedInvoke('start_gesture_capture', { cameraId: resolvedId });
      if (!r2.ok) showGestureError(getUserMessage(r2.error));
    }
  };

  const handleModeChange = async (checked: boolean) => {
    const newMode = checked ? 'gesture' : 'voice';
    const prevMode = props.mode;

    typedInvoke('set_language_mode', { enabled: newMode === 'voice' }).then(r => {
      if (!r.ok) console.error(getUserMessage(r.error));
    });

    if (newMode === 'gesture' && prevMode === 'voice') {
      typedInvoke('start_gesture_capture', { cameraId: selectedCameraId() }).then(r => {
        if (!r.ok) showGestureError(getUserMessage(r.error));
      });
    } else if (newMode === 'voice' && prevMode === 'gesture') {
      // 离开手势模式:
      //   1) 先让后端停止手势线程 (异步,等线程 join)
      //   2) 然后再清空前端 store —— 顺序很重要:
      //      必须在 stop 之后 reset,否则后端最后几个 in-flight
      //      interaction-state-changed 事件会冲掉我们的 reset,把
      //      selected_device 又写回去,蓝点残留 bug 仍然出现
      typedInvoke('stop_gesture_capture').then(r => {
        if (!r.ok) console.error(getUserMessage(r.error));
        resetInteractionContext();
      });
    }

    props.onModeChange?.(newMode);
  };

  const ctx = interactionContext;

  return (
    <div
      class="fixed left-sm right-sm bottom-sm z-control-bar"
      style={{ height: 'var(--control-bar-height)' }}
    >
      <LiquidGlass
        radius={16}
        background="var(--color-glass-bar)"
        overlay={true}
        blur={20}
        edgeBlur={6}
        displacementScale={40}
        contrast={1.15}
        style={{ width: '100%', height: '100%' }}
      >
        <div class="relative z-10 w-full h-full flex items-center justify-between px-xl">
          {/* Left: mode toggle */}
          <LiquidGlassSegmentToggle
            leftLabel={layout.density === 'compact' ? '🎤' : '语言'}
            rightLabel={layout.density === 'compact' ? '👋' : '手势'}
            checked={props.mode === 'gesture'}
            onChange={handleModeChange}
          />

          {/* Center: mic button (voice) or gesture status (gesture) */}
          <Show when={props.mode === 'voice'}>
            <MicButton size={48} />
          </Show>
          <Show when={props.mode === 'gesture'}>
            <div class="flex items-center gap-3">
              <div class="flex flex-col items-center gap-0.5">
                <div
                  class="w-2.5 h-2.5 rounded-full"
                  style={{
                    background: ctx().is_tracking ? 'var(--color-device-fan-on)' : 'var(--color-device-off)',
                    animation: ctx().is_tracking ? 'pulse-glow 1.5s ease-in-out infinite' : 'none',
                  }}
                />
                <span class="text-fine text-tertiary">
                  {ctx().is_tracking ? '检测中' : '待机'}
                </span>
              </div>

              <Show when={ctx().dwell_progress > 0}>
                <div class="flex flex-col items-center gap-0.5">
                  <div class="w-12 h-1 bg-black/10 rounded-full overflow-hidden">
                    <div
                      class="h-full rounded-full transition-[width] duration-100 ease-linear"
                      style={{
                        width: `${ctx().dwell_progress * 100}%`,
                        background: 'var(--color-accent)',
                      }}
                    />
                  </div>
                  <span class="text-fine text-tertiary">
                    {ctx().current_count != null
                      ? (gestureLabels[ctx().current_count!] ?? `${ctx().current_count} 指`)
                      : ''}
                  </span>
                </div>
              </Show>

              <Show when={!ctx().health.camera_ok || !ctx().health.python_ok}>
                <div class="flex items-center gap-1">
                  <div class="w-2 h-2 rounded-full" style={{ background: 'var(--color-log-warn)' }} />
                  <span class="text-fine text-tertiary">
                    {!ctx().health.camera_ok ? '摄像头离线' : 'Python 离线'}
                  </span>
                </div>
              </Show>

              <button
                ref={cameraBtnRef}
                class="flex items-center gap-1 px-2 py-1 rounded-md bg-black/[0.05] hover:bg-black/[0.08] transition-colors"
                onClick={toggleCameraMenu}
                title={selectedCameraId() ?? '选择摄像头'}
              >
                <svg class="w-3.5 h-3.5 text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
                <Show when={layout.density !== 'compact'}>
                  <span class="text-fine text-secondary max-w-[80px] truncate">
                    {selectedCamera()?.name ?? '默认'}
                  </span>
                </Show>
              </button>
            </div>
          </Show>

          {/* Right: view toggle */}
          <div class="flex items-center gap-2">
            <button
              class="btn-icon"
              onClick={() => props.onViewChange?.(props.viewMode === 'devices' ? 'diagnostics' : 'devices')}
              title={props.viewMode === 'devices' ? '切换到诊断' : '切换到设备'}
            >
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                {props.viewMode === 'devices' ? (
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                ) : (
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
                )}
              </svg>
            </button>
          </div>
        </div>
      </LiquidGlass>

      {/* Camera selection menu */}
      <Show when={showCameraMenu()}>
        <Portal>
          <LiquidGlass
            radius={12}
            background="var(--color-glass-surface)"
            overlay={true}
            blur={15}
            edgeBlur={5}
            displacementScale={35}
            contrast={1.2}
            class="z-floating"
            style={{
              position: 'fixed',
              bottom: cameraMenuPos().bottom,
              left: cameraMenuPos().left,
              'min-width': '180px',
            }}
          >
            <div data-camera-menu class="py-1">
              <button
                class="w-full text-left px-3 py-2 text-caption transition-colors flex items-center gap-2 hover:bg-black/[0.05]"
                style={{ color: !selectedCameraId() ? 'var(--color-accent)' : 'var(--color-text-primary)' }}
                onClick={() => selectCamera('')}
              >
                默认摄像头
              </button>
              <For each={cameras()}>
                {(cam) => (
                  <button
                    class="w-full text-left px-3 py-2 text-caption transition-colors flex items-center gap-2 hover:bg-black/[0.05]"
                    style={{ color: selectedCameraId() === cam.id ? 'var(--color-accent)' : 'var(--color-text-primary)' }}
                    onClick={() => selectCamera(cam.id)}
                  >
                    <span
                      class="w-2 h-2 rounded-full flex-shrink-0"
                      style={{ background: selectedCameraId() === cam.id ? 'var(--color-accent)' : 'var(--color-border-strong)' }}
                    />
                    <span class="truncate">{cam.name}</span>
                  </button>
                )}
              </For>
            </div>
          </LiquidGlass>
        </Portal>
      </Show>

      {/* 手势启动错误横幅(修复 A):位置在 ControlBar 正上方,5s 自动消失,可手动关闭 */}
      <Show when={gestureError()}>
        <Portal>
          <div
            class="fixed left-sm right-sm z-toast pointer-events-none"
            style={{
              bottom: 'calc(var(--control-bar-height) + 12px + var(--spacing-sm, 8px))',
            }}
          >
            <div
              class="pointer-events-auto mx-auto flex items-center gap-2 px-4 py-2.5 rounded-xl shadow-lg max-w-md"
              style={{
                background: 'rgba(255, 59, 48, 0.95)',
                color: 'white',
                'backdrop-filter': 'blur(12px)',
                '-webkit-backdrop-filter': 'blur(12px)',
              }}
            >
              <svg class="w-4 h-4 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z" clip-rule="evenodd" />
              </svg>
              <span class="text-caption flex-1">{gestureError()}</span>
              <button
                class="flex-shrink-0 opacity-80 hover:opacity-100 transition-opacity"
                onClick={() => {
                  setGestureError(null);
                  if (gestureErrorTimer !== undefined) clearTimeout(gestureErrorTimer);
                }}
                aria-label="关闭"
              >
                <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
                </svg>
              </button>
            </div>
          </div>
        </Portal>
      </Show>
    </div>
  );
}
