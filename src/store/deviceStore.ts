// 设备状态管理 - 待实现
import { createSignal } from 'solid-js';

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

export { deviceState, setDeviceState };
