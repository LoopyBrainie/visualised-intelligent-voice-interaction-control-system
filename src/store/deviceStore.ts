// 设备状态管理
import { createSignal } from 'solid-js';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export type RoomId = 'living_room' | 'bedroom';
export type DeviceType = 'light' | 'air_condition' | 'fan' | 'curtain';

// 单个设备在不同房间的状态
export interface LightState {
  is_on: boolean;
  brightness: number;
}

export interface AirConditionState {
  is_on: boolean;
  temperature: number;
  mode: string;
}

export interface FanState {
  is_on: boolean;
  speed: number;
}

export interface CurtainState {
  is_open: boolean;
}

// 单个房间内的设备状态
export interface RoomDeviceState {
  light: LightState;
  air_condition: AirConditionState;
  fan: FanState;
  curtain: CurtainState;
}

// 全局设备状态（按房间组织）
export interface DeviceState {
  living_room: RoomDeviceState;
  bedroom: RoomDeviceState;
}

// 创建默认房间状态
const createDefaultRoomState = (): RoomDeviceState => ({
  light: { is_on: false, brightness: 50 },
  air_condition: { is_on: false, temperature: 24, mode: 'cool' },
  fan: { is_on: false, speed: 1 },
  curtain: { is_open: false },
});

const [deviceState, setDeviceState] = createSignal<DeviceState>({
  living_room: createDefaultRoomState(),
  bedroom: createDefaultRoomState(),
});

// 初始同步 - 应用启动时拉取 Rust 端当前状态
export async function syncInitialState() {
  try {
    const stateJson = await invoke<string>('get_device_state');
    const state = JSON.parse(stateJson) as DeviceState;
    setDeviceState(state);
  } catch (e) {
    console.error('状态同步失败:', e);
  }
}

// 监听后端状态更新事件
// 后端通过 device-state-changed 事件广播状态变化
listen('device-state-changed', (event) => {
  const payload = event.payload as Partial<DeviceState>;
  setDeviceState(prev => ({ ...prev, ...payload }));
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
