// 设备状态管理
import { createSignal } from 'solid-js';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export interface DeviceState {
  light: boolean;
  light_brightness: number;
  air_condition: boolean;
  ac_temp: number;
  fan: boolean;
  fan_speed: number;
  curtain: boolean;
}

const [deviceState, setDeviceState] = createSignal<DeviceState>({
  light: false,
  light_brightness: 50,
  air_condition: false,
  ac_temp: 24,
  fan: false,
  fan_speed: 1,
  curtain: false,
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

export { deviceState, setDeviceState };
