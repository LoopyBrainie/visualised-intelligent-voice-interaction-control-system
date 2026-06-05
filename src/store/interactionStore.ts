// interactionStore.ts - 手势交互状态管理
// 职责: 监听 Rust 端 interaction-state-changed 事件，管理手势交互上下文

import { createSignal } from 'solid-js';
import { listen } from '@tauri-apps/api/event';

export interface GestureHealth {
  camera_ok: boolean;
  python_ok: boolean;
  camera_name: string | null;
}

export interface Landmark {
  x: number;
  y: number;
  z: number;
}

export interface InteractionContext {
  selected_device: string;
  current_count: number | null;
  dwell_progress: number;
  dwell_deadline_ms: number | null;
  is_tracking: boolean;
  last_action: string | null;
  health: GestureHealth;
  landmarks: Landmark[];
}

const defaultContext: InteractionContext = {
  selected_device: 'light',
  current_count: null,
  dwell_progress: 0,
  dwell_deadline_ms: null,
  is_tracking: false,
  last_action: null,
  health: { camera_ok: false, python_ok: false, camera_name: null },
  landmarks: [],
};

const [interactionContext, setInteractionContext] = createSignal<InteractionContext>(defaultContext);

/// 重置手势交互上下文 —— 在离开手势模式时调用,避免 selection / dwell 等
/// 模式专属状态泄漏到语言模式的视图(典型表现:设备卡片右上角蓝点
/// 在切回语言模式后仍然亮着)。
///
/// 设计决策:保留 `health` 字段 —— 它反映物理硬件状态(摄像头/Python
/// daemon 是否在线),与"用户当前在哪个模式"正交。若在切模式时被
/// 强制清成"未连接",用户再切回手势模式时会有几十毫秒的假阴性窗口。
export function resetInteractionContext() {
  setInteractionContext(prev => ({ ...defaultContext, health: prev.health }));
}

// 监听后端交互状态更新
listen<InteractionContext>('interaction-state-changed', (event) => {
  setInteractionContext(prev => ({ ...prev, ...event.payload }));
});

// 监听健康状态更新（Rust 端在连续识别失败时单独发射此事件）
listen<GestureHealth>('gesture-health-changed', (event) => {
  setInteractionContext(prev => ({ ...prev, health: event.payload }));
});

// 监听手势动作确认事件
const [lastGestureAction, setLastGestureAction] = createSignal<string | null>(null);

listen<string>('gesture-action-confirmed', (event) => {
  setLastGestureAction(event.payload);
  // 3 秒后自动清除
  setTimeout(() => setLastGestureAction(null), 3000);
});

export { interactionContext, lastGestureAction };
