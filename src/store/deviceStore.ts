// 设备状态管理
// 状态契约 (字段名 + 字面量类型) 由 arktype schema 统一维护 ——
// 任何 Rust 端字段重命名、类型变更 (e.g. snake_case → camelCase 漂移)
// 都会被 schema 拒绝并显式 console.error 记录,
// 而非悄悄塞进 store 让下游组件踩到 undefined。
import { createSignal } from 'solid-js';
import { listen } from '@tauri-apps/api/event';
import { type } from 'arktype';
import { typedInvoke, getUserMessage } from '@/errors';

export type RoomId = 'living_room' | 'bedroom';
export type DeviceType = 'light' | 'air_condition' | 'fan' | 'curtain';

// 单个房间内的设备状态。键名必须与 Rust 端 serde 序列化键一致
// (snake_case, 无 rename_all 指令)。
// Optional 字段对应 Rust 端 `#[serde(default)] Option<T>`:
// 序列化为 `null` 或省略, 组件目前不消费, 允许任意值以保持向后兼容。
const roomDeviceSchema = type({
  light: {
    is_on: 'boolean',
    brightness: 'number',
    'color_temp?': 'number | null',
  },
  air_condition: {
    is_on: 'boolean',
    temperature: 'number',
    // 锁定 ACMode 枚举字面量 —— 后端改名会立刻被此 schema 拒绝
    mode: "'Cool' | 'Heat' | 'Auto' | 'Fan'",
    'swing?': 'boolean | null',
  },
  fan: {
    is_on: 'boolean',
    speed: 'number',
    'oscillate?': 'boolean | null',
  },
  curtain: {
    is_open: 'boolean',
    'position?': 'number | null',
  },
});

export const deviceStateSchema = type({
  living_room: roomDeviceSchema,
  bedroom: roomDeviceSchema,
});

export type RoomDeviceState = typeof roomDeviceSchema.infer;
export type DeviceState = typeof deviceStateSchema.infer;

// 创建默认房间状态 — 必须与 Rust 端 `Default for RoomDeviceState` 保持一致，
// 否则在 syncInitialState 失败（如 Rust 锁中毒）时，前端会显示错误的初始值。
// mode 使用 PascalCase 以匹配 Rust 端 ACMode 序列化字面量
const createDefaultRoomState = (): RoomDeviceState => ({
  light: { is_on: false, brightness: 100 },
  air_condition: { is_on: false, temperature: 24, mode: 'Cool' },
  fan: { is_on: false, speed: 0 },
  curtain: { is_open: false },
});

const [deviceState, setDeviceState] = createSignal<DeviceState>({
  living_room: createDefaultRoomState(),
  bedroom: createDefaultRoomState(),
});

// 初始同步 - 应用启动时拉取 Rust 端当前状态
export async function syncInitialState() {
  const result = await typedInvoke<string>('get_device_state');
  if (!result.ok) {
    console.error(getUserMessage(result.error));
    return;
  }
  // Rust 在极端情况下（panic 恢复期、序列化器版本不匹配）可能返回非 JSON 字符串，
  // 保护性 parse 避免把异常向上抛成未处理的 promise rejection
  let raw: unknown;
  try {
    raw = JSON.parse(result.value);
  } catch (e) {
    console.error('[deviceStore] 解析设备状态失败，保留默认值:', e);
    return;
  }
  const validated = deviceStateSchema(raw);
  if (validated instanceof type.errors) {
    // 序列化契约不一致 (Rust 字段重命名 / 类型变更 / 缺字段) ——
    // 保留上次已知良好状态, 避免被畸形结构污染 reactive 节点
    console.error('[deviceStore] 设备状态结构不匹配，保留默认值:', validated.summary);
    return;
  }
  setDeviceState(validated);
}

// 监听后端状态更新事件
// 后端通过 device-state-changed 事件广播完整状态
listen('device-state-changed', (event) => {
  const validated = deviceStateSchema(event.payload);
  if (validated instanceof type.errors) {
    // 拒绝坏载荷: 保留当前状态, 避免 reactive 节点被污染,
    // 防止下游组件触发 `TypeError: Cannot read properties of undefined`
    console.error('[deviceStore] 设备状态事件结构不匹配，已忽略:', validated.summary);
    return;
  }
  // 完整替换而非浅合并: Rust 端只发送完整状态 (lib.rs:259),
  // 浅合并会在后端补发新字段时悄悄丢掉其它房间/设备状态。
  setDeviceState(validated);
});

// 辅助函数：获取指定房间的设备状态
export function getRoomState(roomId: RoomId) {
  return () => deviceState()[roomId];
}

// 辅助函数：获取指定房间的指定设备状态
export function getDeviceState(roomId: RoomId, device: DeviceType) {
  return () => deviceState()[roomId][device];
}

export { deviceState, setDeviceState };
