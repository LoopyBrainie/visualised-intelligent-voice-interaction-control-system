// state.rs - 全局状态模块 - 待实现
// 职责: 设备状态定义、序列化/反序列化
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceState