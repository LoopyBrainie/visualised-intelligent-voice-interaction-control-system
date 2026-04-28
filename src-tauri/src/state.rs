// state.rs - 设备状态模块
// 职责: 设备状态定义、序列化/反序列化、线程安全封装
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

// ============================================================
// 空调模式枚举
// ============================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ACMode {
    Cool,
    Heat,
    Auto,
    Fan,
}

impl Default for ACMode {
    fn default() -> Self {
        ACMode::Cool
    }
}

// ============================================================
// 子设备状态结构
// ============================================================

/// 灯光状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightState {
    pub is_on: bool,
    pub brightness: u8, // 0-100
    #[serde(default)]
    pub color_temp: Option<u8>, // 预留: 色温 2700K-6500K
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            is_on: false,
            brightness: 100,
            color_temp: None,
        }
    }
}

/// 空调状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirConditionState {
    pub is_on: bool,
    pub temperature: u8, // 16-30
    pub mode: ACMode,
    #[serde(default)]
    pub swing: Option<bool>, // 预留: 摆风
}

impl Default for AirConditionState {
    fn default() -> Self {
        Self {
            is_on: false,
            temperature: 24,
            mode: ACMode::Cool,
            swing: None,
        }
    }
}

/// 风扇状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanState {
    pub is_on: bool,
    pub speed: u8, // 0-3
    #[serde(default)]
    pub oscillate: Option<bool>, // 预留: 摇头
}

impl Default for FanState {
    fn default() -> Self {
        Self {
            is_on: false,
            speed: 0,
            oscillate: None,
        }
    }
}

// ============================================================
// 设备状态组合结构
// ============================================================

/// 设备状态组合结构（便于 serde 序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub light: LightState,
    pub air_condition: AirConditionState,
    pub fan: FanState,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            light: LightState::default(),
            air_condition: AirConditionState::default(),
            fan: FanState::default(),
        }
    }
}

// ============================================================
// 全局状态（线程安全封装）
// ============================================================

/// 全局状态（线程安全）
/// 使用 Arc<RwLock<DeviceState>> 保护内部状态，支持多线程并发读写
#[derive(Debug, Clone)]
pub struct GlobalState {
    inner: Arc<RwLock<DeviceState>>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(DeviceState::default())),
        }
    }

    /// 获取读锁（用于查看状态）
    pub fn read(&self) -> Result<std::sync::RwLockReadGuard<'_, DeviceState>, std::sync::PoisonError<std::sync::RwLockReadGuard<'_, DeviceState>>> {
        self.inner.read()
    }

    /// 获取写锁（用于修改状态）
    pub fn write(&self) -> Result<std::sync::RwLockWriteGuard<'_, DeviceState>, std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, DeviceState>>> {
        self.inner.write()
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_state_default() {
        let light = LightState::default();
        assert!(!light.is_on);
        assert_eq!(light.brightness, 100);
        assert!(light.color_temp.is_none());
    }

    #[test]
    fn test_ac_state_default() {
        let ac = AirConditionState::default();
        assert!(!ac.is_on);
        assert_eq!(ac.temperature, 24);
        assert_eq!(ac.mode, ACMode::Cool);
    }

    #[test]
    fn test_fan_state_default() {
        let fan = FanState::default();
        assert!(!fan.is_on);
        assert_eq!(fan.speed, 0);
    }

    #[test]
    fn test_device_state_default() {
        let state = DeviceState::default();
        assert!(!state.light.is_on);
        assert!(!state.air_condition.is_on);
        assert!(!state.fan.is_on);
    }

    #[test]
    fn test_device_state_serialization() {
        let state = DeviceState::default();
        let json = serde_json::to_string(&state).expect("序列化失败");
        let back: DeviceState = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(state.light.is_on, back.light.is_on);
        assert_eq!(state.light.brightness, back.light.brightness);
        assert_eq!(state.air_condition.temperature, back.air_condition.temperature);
    }

    #[test]
    fn test_global_state_creation() {
        let global = GlobalState::new();
        // 验证初始状态
        let guard = global.read().expect("获取读锁失败");
        assert!(!guard.light.is_on);
        drop(guard);

        // 验证写锁可用
        let mut guard = global.write().expect("获取写锁失败");
        guard.light.is_on = true;
        drop(guard);

        // 验证更新后状态
        let guard = global.read().expect("获取读锁失败");
        assert!(guard.light.is_on);
    }
}
