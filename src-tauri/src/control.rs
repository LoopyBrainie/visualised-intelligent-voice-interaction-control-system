// control.rs - 指令解析与设备控制模块
// 职责: 指令字典、指令解析、设备状态更新、VLM 指令解析
use crate::state::{AirConditionState, GlobalState, LightState, FanState, RoomDeviceState};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use strsim::jaro_winkler;
use thiserror::Error;

// ============================================================
// VLM 指令解析类型定义 (C1, C4)
// ============================================================

/// VLM 错误类型
#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("VLM 请求超时")]
    VlmTimeout,
    #[error("VLM 响应解析失败: {0}")]
    VlmParseError(String),
    #[error("VLM 置信度低: {0:.2} < 0.6")]
    VlmLowConfidence(f64),
    #[error("设备执行失败: {0}")]
    ExecuteError(String),
    #[error("无法识别指令")]
    UnknownCommand,
}

/// VLM API 返回的 JSON 结构（Python Daemon 输出）
/// 格式: {"command": {"device": "light", "action": "on", ...}, "raw_text": "..."}
#[derive(Deserialize, Debug)]
pub struct VlmResponse {
    pub command: VlmCommandPayload,
    #[serde(default)]
    pub raw_text: Option<String>,
}

/// VLM Command Payload (Python Daemon 返回的 command 字段)
#[derive(Deserialize, Debug)]
pub struct VlmCommandPayload {
    pub device: String,
    pub action: String,
    #[serde(default)]
    pub room: Option<String>,  // 房间信息: "living_room" | "bedroom"
    #[serde(default)]
    pub target_value: Option<f64>,
    #[serde(default)]
    pub confidence: Option<f64>,
}

/// VLM 指令转换后的内部格式
#[derive(Debug)]
pub struct VlmCommand {
    pub device: TargetDevice,
    pub action: CommandType,
    pub room: Option<String>,
    pub value: Option<u8>,
}

impl TryFrom<VlmResponse> for VlmCommand {
    type Error = VoiceError;

    fn try_from(vlm: VlmResponse) -> Result<Self, Self::Error> {
        // 将 daemon action 格式转换为 CommandType
        let action = match vlm.command.action.as_str() {
            "on" | "turn_on" | "open" => CommandType::TurnOn,
            "off" | "turn_off" | "close" => CommandType::TurnOff,
            "set_level" | "set_value" => CommandType::SetValue,
            "auto" => CommandType::SetValue, // 空调自动模式映射到 SetValue
            "query" => CommandType::Query,
            _ => {
                return Err(VoiceError::VlmParseError(format!(
                    "未知的 action 类型: {}",
                    vlm.command.action
                )))
            }
        };

        let device = match vlm.command.device.as_str() {
            "light" => TargetDevice::Light,
            "fan" => TargetDevice::Fan,
            "ac" | "air_condition" | "aircondition" => TargetDevice::AirCondition,
            "curtain" => TargetDevice::Curtain, // 新增窗帘支持
            "all" => TargetDevice::All,
            _ => {
                return Err(VoiceError::VlmParseError(format!(
                    "未知的 device 类型: {}",
                    vlm.command.device
                )))
            }
        };

        // 从 target_value 提取数值
        let value = vlm.command.target_value.map(|v| v as u8);

        // 使用 room 字段（如果提供的话）
        let room = vlm.command.room.clone();

        Ok(VlmCommand {
            device,
            action,
            room,
            value,
        })
    }
}

/// VLM JSON 解析入口
pub fn parse_vlm_response(json: &str) -> Result<VlmCommand, VoiceError> {
    let vlm: VlmResponse =
        serde_json::from_str(json).map_err(|e| VoiceError::VlmParseError(e.to_string()))?;

    // 置信度检查 (可选)
    if let Some(conf) = vlm.command.confidence {
        if conf < 0.6 {
            return Err(VoiceError::VlmLowConfidence(conf));
        }
    }

    vlm.try_into()
}

/// 执行 VLM 指令
pub fn execute_vlm_command(cmd: &VlmCommand, state: &GlobalState) -> Result<(), VoiceError> {
    // 将 VLM 的房间字符串转换为 TargetRoom
    let room = match cmd.room.as_deref() {
        Some("bedroom") => TargetRoom::Bedroom,
        Some("living_room") | Some("livingroom") => TargetRoom::LivingRoom,
        _ => TargetRoom::LivingRoom,
    };

    let parsed = ParsedCommand {
        cmd_type: cmd.action,
        target: cmd.device,
        value: cmd.value,
        room,
    };
    execute_command(&parsed, state)
        .map_err(|e| VoiceError::ExecuteError(e.to_string()))
}

// ============================================================
// 指令相关类型定义
// ============================================================

/// 指令类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    TurnOn,
    TurnOff,
    SetValue,
    Query,
}

/// 目标设备
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetDevice {
    Light,
    AirCondition,
    Fan,
    Curtain,
    All,
}

/// 目标房间
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetRoom {
    LivingRoom,
    Bedroom,
    #[allow(dead_code)]
    All,
}

impl Default for TargetRoom {
    fn default() -> Self {
        TargetRoom::LivingRoom // 默认客厅
    }
}

/// 解析后的指令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub cmd_type: CommandType,
    pub target: TargetDevice,
    pub value: Option<u8>, // 用于 SetValue (温度/亮度/风速)
    pub room: TargetRoom,  // 目标房间
}

/// 控制错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlError {
    Unrecognized(String),
    InvalidValue { expected: String, got: u8 },
    DeviceNotFound(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::Unrecognized(s) => write!(f, "无法识别的指令: {}", s),
            ControlError::InvalidValue { expected, got } => {
                write!(f, "无效数值: 期望 {}，实际 {}", expected, got)
            }
            ControlError::DeviceNotFound(s) => write!(f, "设备未找到: {}", s),
        }
    }
}

impl std::error::Error for ControlError {}

// ============================================================
// 指令字典（精确匹配）
// ============================================================

type CommandEntry = (CommandType, TargetDevice, Option<u8>);

lazy_static! {
    /// 指令字典: 指令模板 → (命令类型, 目标设备, 数值)
    /// 数值 None 表示无参数，Some(n) 表示该命令需要参数
    static ref COMMAND_DICT: HashMap<&'static str, CommandEntry> = {
        let mut m = HashMap::new();

        // ===== 灯 - 开启 =====
        m.insert("开灯", (CommandType::TurnOn, TargetDevice::Light, None));
        m.insert("打开灯", (CommandType::TurnOn, TargetDevice::Light, None));
        m.insert("把灯打开", (CommandType::TurnOn, TargetDevice::Light, None));
        m.insert("灯打开", (CommandType::TurnOn, TargetDevice::Light, None));
        m.insert("开灯吧", (CommandType::TurnOn, TargetDevice::Light, None));

        // ===== 灯 - 关闭 =====
        m.insert("关灯", (CommandType::TurnOff, TargetDevice::Light, None));
        m.insert("把灯关上", (CommandType::TurnOff, TargetDevice::Light, None));
        m.insert("灯关闭", (CommandType::TurnOff, TargetDevice::Light, None));
        m.insert("关灯吧", (CommandType::TurnOff, TargetDevice::Light, None));

        // ===== 空调 - 开启/关闭 =====
        m.insert("开空调", (CommandType::TurnOn, TargetDevice::AirCondition, None));
        m.insert("打开空调", (CommandType::TurnOn, TargetDevice::AirCondition, None));
        m.insert("把空调打开", (CommandType::TurnOn, TargetDevice::AirCondition, None));
        m.insert("关空调", (CommandType::TurnOff, TargetDevice::AirCondition, None));
        m.insert("把空调关上", (CommandType::TurnOff, TargetDevice::AirCondition, None));

        // ===== 风扇 - 开启/关闭 =====
        m.insert("开风扇", (CommandType::TurnOn, TargetDevice::Fan, None));
        m.insert("打开风扇", (CommandType::TurnOn, TargetDevice::Fan, None));
        m.insert("把风扇打开", (CommandType::TurnOn, TargetDevice::Fan, None));
        m.insert("关风扇", (CommandType::TurnOff, TargetDevice::Fan, None));
        m.insert("把风扇关上", (CommandType::TurnOff, TargetDevice::Fan, None));

        // ===== 全局控制 =====
        m.insert("全部打开", (CommandType::TurnOn, TargetDevice::All, None));
        m.insert("全部开启", (CommandType::TurnOn, TargetDevice::All, None));
        m.insert("打开所有", (CommandType::TurnOn, TargetDevice::All, None));
        m.insert("全部关闭", (CommandType::TurnOff, TargetDevice::All, None));
        m.insert("关闭所有", (CommandType::TurnOff, TargetDevice::All, None));

        m
    };

    /// 动态数值提取正则模式
    /// (正则表达式, 目标设备)
    static ref VALUE_PATTERNS: Vec<(&'static str, TargetDevice)> = vec![
        // 空调温度设置: "空调调到25度", "空调温度26度", "把空调调到20度"
        (r"空调.*?(\d{2})度", TargetDevice::AirCondition),
        (r"空调调[节到](\d{2})", TargetDevice::AirCondition),
        (r"空调温度(\d{2})", TargetDevice::AirCondition),
        (r"温度.*?(\d{2})度", TargetDevice::AirCondition),
        (r"调[节到]空调(\d{2})度", TargetDevice::AirCondition),

        // 灯光亮度设置: "灯亮度调到80", "灯光调暗一点" (预留)
        (r"灯.*?亮度.*?(\d{1,3})", TargetDevice::Light),
        (r"灯光.*?(\d{1,3})", TargetDevice::Light),

        // 风扇档位设置: "风扇1档", "风扇中速"
        (r"风扇.*?(\d)档", TargetDevice::Fan),
        (r"风扇.*?[低中高]速", TargetDevice::Fan),
    ];
}

// ============================================================
// 指令解析函数
// ============================================================

/// 解析语音指令
///
/// 处理流程:
/// 1. 预处理: 去空白、小写化
/// 2. 提取房间: 从指令中识别"卧室"、"客厅"
/// 3. 精确匹配: 查找 COMMAND_DICT
/// 4. 正则提取: 动态数值（如温度、亮度）
/// 5. 模糊匹配 fallback: 通过 PyO3 调用 Python rapidfuzz
///
/// # Arguments
/// * `input` - 原始语音指令字符串
///
/// # Returns
/// * `Ok(ParsedCommand)` - 解析成功
/// * `Err(ControlError)` - 解析失败
pub fn parse_command(input: &str) -> Result<ParsedCommand, ControlError> {
    let normalized = input.trim().to_lowercase();

    // Step 1: 提取房间信息
    let room = extract_room(&normalized);

    // Step 2: 去除房间前缀，得到纯指令
    let cmd_stripped = strip_room_prefix(&normalized);

    // Step 3: 精确匹配
    if let Some(entry) = COMMAND_DICT.get(cmd_stripped.as_str()) {
        return Ok(ParsedCommand {
            cmd_type: entry.0,
            target: entry.1,
            value: entry.2,
            room,
        });
    }

    // Step 4: 正则提取数值
    for (pattern, target) in VALUE_PATTERNS.iter() {
        if let Some(caps) = Regex::new(pattern)
            .ok()
            .and_then(|r| r.captures(&cmd_stripped))
        {
            if let Some(val_str) = caps.get(1) {
                if let Ok(val) = val_str.as_str().parse::<u8>() {
                    return Ok(ParsedCommand {
                        cmd_type: CommandType::SetValue,
                        target: *target,
                        value: Some(val),
                        room,
                    });
                }
            }
        }
    }

    // Step 4: Rust 原生模糊匹配 fallback (C3)
    match fuzzy_match_command(&normalized) {
        Some((action, target, value)) => Ok(ParsedCommand {
            cmd_type: action,
            target,
            value,
            room,
        }),
        None => Err(ControlError::Unrecognized(input.to_string())),
    }
}

/// 从指令中提取目标房间
fn extract_room(input: &str) -> TargetRoom {
    // 优先匹配卧室
    if input.contains("卧室") || input.contains("卧房") || input.contains("睡房") {
        return TargetRoom::Bedroom;
    }
    // 其次匹配客厅
    if input.contains("客厅") || input.contains("厅") {
        return TargetRoom::LivingRoom;
    }
    // 默认客厅
    TargetRoom::LivingRoom
}

/// 去除指令中的房间前缀/后缀，尝试多种组合找到匹配的指令
fn strip_room_prefix(input: &str) -> String {
    // 房间词列表
    let room_words = ["卧室", "卧房", "睡房", "客厅", "厅"];
    // 命令前缀
    let cmd_prefixes = ["打开", "关闭", "把"];

    // 尝试多种组合
    let attempts = [
        input.to_string(),
        // 先去命令前缀
        strip_prefix_once(input, &cmd_prefixes),
        // 先去房间前缀
        strip_prefix_once(input, &room_words),
        // 先去命令前缀，再去房间前缀
        strip_prefix_once(&strip_prefix_once(input, &cmd_prefixes), &room_words),
        // 先去房间前缀，再去命令前缀
        strip_prefix_once(&strip_prefix_once(input, &room_words), &cmd_prefixes),
        // 去除房间词（从任意位置）
        strip_room_word(input, &room_words),
        // 去除房间词后再去命令前缀
        strip_prefix_once(&strip_room_word(input, &room_words), &cmd_prefixes),
    ];

    // 返回第一个在 COMMAND_DICT 中存在的组合
    for attempt in &attempts {
        let trimmed = attempt.trim();
        if !trimmed.is_empty() && COMMAND_DICT.contains_key(trimmed) {
            return trimmed.to_string();
        }
    }

    // 如果都不匹配，返回原始输入（用于模糊匹配）
    input.to_string()
}

/// 从字符串开头去除指定的任意一个前缀
fn strip_prefix_once<'a>(input: &'a str, prefixes: &[&str]) -> String {
    for prefix in prefixes {
        if input.starts_with(prefix) {
            return input[prefix.len()..].to_string();
        }
    }
    input.to_string()
}

/// 从字符串中去除房间词（任意位置）
fn strip_room_word(input: &str, room_words: &[&str]) -> String {
    let mut result = input.to_string();
    for room in room_words {
        result = result.replace(room, "");
    }
    result
}

// ============================================================
// 模糊匹配降级 (C3 - strsim crate)
// ============================================================

lazy_static! {
    /// 模糊匹配关键词表: (指令文本, 命令类型, 目标设备)
    static ref FUZZY_KEYWORDS: Vec<(&'static str, CommandType, TargetDevice)> = vec![
        // 灯 - 开启
        ("开灯", CommandType::TurnOn, TargetDevice::Light),
        ("打开灯", CommandType::TurnOn, TargetDevice::Light),
        ("把灯打开", CommandType::TurnOn, TargetDevice::Light),
        ("灯打开", CommandType::TurnOn, TargetDevice::Light),
        ("开灯吧", CommandType::TurnOn, TargetDevice::Light),
        // 灯 - 关闭
        ("关灯", CommandType::TurnOff, TargetDevice::Light),
        ("把灯关上", CommandType::TurnOff, TargetDevice::Light),
        ("灯关闭", CommandType::TurnOff, TargetDevice::Light),
        ("关灯吧", CommandType::TurnOff, TargetDevice::Light),
        // 空调 - 开启
        ("开空调", CommandType::TurnOn, TargetDevice::AirCondition),
        ("打开空调", CommandType::TurnOn, TargetDevice::AirCondition),
        ("把空调打开", CommandType::TurnOn, TargetDevice::AirCondition),
        // 空调 - 关闭
        ("关空调", CommandType::TurnOff, TargetDevice::AirCondition),
        ("把空调关上", CommandType::TurnOff, TargetDevice::AirCondition),
        // 风扇 - 开启
        ("开风扇", CommandType::TurnOn, TargetDevice::Fan),
        ("打开风扇", CommandType::TurnOn, TargetDevice::Fan),
        ("把风扇打开", CommandType::TurnOn, TargetDevice::Fan),
        // 风扇 - 关闭
        ("关风扇", CommandType::TurnOff, TargetDevice::Fan),
        ("把风扇关上", CommandType::TurnOff, TargetDevice::Fan),
        // 窗帘 - 开启
        ("开窗帘", CommandType::TurnOn, TargetDevice::Curtain),
        ("打开窗帘", CommandType::TurnOn, TargetDevice::Curtain),
        ("拉开窗帘", CommandType::TurnOn, TargetDevice::Curtain),
        // 窗帘 - 关闭
        ("关窗帘", CommandType::TurnOff, TargetDevice::Curtain),
        ("把窗帘关上", CommandType::TurnOff, TargetDevice::Curtain),
        ("关闭窗帘", CommandType::TurnOff, TargetDevice::Curtain),
        // 全局控制
        ("全部打开", CommandType::TurnOn, TargetDevice::All),
        ("全部开启", CommandType::TurnOn, TargetDevice::All),
        ("打开所有", CommandType::TurnOn, TargetDevice::All),
        ("全部关闭", CommandType::TurnOff, TargetDevice::All),
        ("关闭所有", CommandType::TurnOff, TargetDevice::All),
    ];
}

/// Rust 原生模糊匹配（使用 strsim jaro_winkler 算法）
///
/// # Arguments
/// * `input` - 标准化后的指令字符串
///
/// # Returns
/// * `Some((action, target, value))` - 匹配成功，相似度 > 80%
/// * `None` - 匹配失败
fn fuzzy_match_command(input: &str) -> Option<(CommandType, TargetDevice, Option<u8>)> {
    let mut best: Option<(CommandType, TargetDevice, Option<u8>, f64)> = None;

    for (keyword, action, target) in FUZZY_KEYWORDS.iter() {
        let score = jaro_winkler(input, keyword);
        if score > 0.8 {
            match best {
                None => best = Some((*action, *target, None, score)),
                Some((_, _, _, best_score)) if score > best_score => {
                    best = Some((*action, *target, None, score));
                }
                _ => {}
            }
        }
    }

    best.map(|(action, target, value, _)| (action, target, value))
}

// ============================================================
// 设备状态更新函数
// ============================================================

/// 执行指令并更新状态
///
/// # Arguments
/// * `cmd` - 解析后的指令
/// * `state` - 全局状态
///
/// # Returns
/// * `Ok(())` - 执行成功
/// * `Err(ControlError)` - 执行失败
pub fn execute_command(cmd: &ParsedCommand, state: &GlobalState) -> Result<(), ControlError> {
    // 获取写锁（用于多线程保护）
    let mut guard = state
        .write()
        .map_err(|_| ControlError::DeviceNotFound("Lock poisoned".into()))?;

    // 根据目标房间执行指令
    match cmd.room {
        TargetRoom::LivingRoom => apply_command_to_room(cmd, &mut guard.living_room),
        TargetRoom::Bedroom => apply_command_to_room(cmd, &mut guard.bedroom),
        TargetRoom::All => {
            apply_command_to_room(cmd, &mut guard.living_room)?;
            apply_command_to_room(cmd, &mut guard.bedroom)
        }
    }
}

/// 将指令应用到指定房间
fn apply_command_to_room(cmd: &ParsedCommand, room: &mut RoomDeviceState) -> Result<(), ControlError> {
    match cmd.target {
        TargetDevice::Light => apply_light(cmd, &mut room.light),
        TargetDevice::AirCondition => apply_ac(cmd, &mut room.air_condition),
        TargetDevice::Fan => apply_fan(cmd, &mut room.fan),
        TargetDevice::Curtain => apply_curtain(cmd, &mut room.curtain),
        TargetDevice::All => {
            apply_light(cmd, &mut room.light)?;
            apply_ac(cmd, &mut room.air_condition)?;
            apply_fan(cmd, &mut room.fan)?;
            apply_curtain(cmd, &mut room.curtain)?;
            Ok(())
        }
    }
}

/// 应用灯光指令
fn apply_light(cmd: &ParsedCommand, light: &mut LightState) -> Result<(), ControlError> {
    match cmd.cmd_type {
        CommandType::TurnOn => {
            light.is_on = true;
        }
        CommandType::TurnOff => {
            light.is_on = false;
        }
        CommandType::SetValue => {
            if let Some(v) = cmd.value {
                if v > 100 {
                    return Err(ControlError::InvalidValue {
                        expected: "0-100".into(),
                        got: v,
                    });
                }
                light.brightness = v;
            }
        }
        CommandType::Query => {
            // 查询操作不修改状态
        }
    }
    Ok(())
}

/// 应用空调指令
fn apply_ac(cmd: &ParsedCommand, ac: &mut AirConditionState) -> Result<(), ControlError> {
    match cmd.cmd_type {
        CommandType::TurnOn => {
            ac.is_on = true;
        }
        CommandType::TurnOff => {
            ac.is_on = false;
        }
        CommandType::SetValue => {
            if let Some(v) = cmd.value {
                if !(16..=30).contains(&v) {
                    return Err(ControlError::InvalidValue {
                        expected: "16-30".into(),
                        got: v,
                    });
                }
                ac.temperature = v;
            }
        }
        CommandType::Query => {}
    }
    Ok(())
}

/// 应用风扇指令
fn apply_fan(cmd: &ParsedCommand, fan: &mut FanState) -> Result<(), ControlError> {
    match cmd.cmd_type {
        CommandType::TurnOn => {
            fan.is_on = true;
        }
        CommandType::TurnOff => {
            fan.is_on = false;
        }
        CommandType::SetValue => {
            if let Some(v) = cmd.value {
                if v > 3 {
                    return Err(ControlError::InvalidValue {
                        expected: "0-3".into(),
                        got: v,
                    });
                }
                fan.speed = v;
            }
        }
        CommandType::Query => {}
    }
    Ok(())
}

/// 应用窗帘指令
fn apply_curtain(cmd: &ParsedCommand, curtain: &mut crate::state::CurtainState) -> Result<(), ControlError> {
    match cmd.cmd_type {
        CommandType::TurnOn | CommandType::SetValue => {
            // TurnOn 和 SetValue 都映射为 open
            curtain.is_open = true;
        }
        CommandType::TurnOff => {
            curtain.is_open = false;
        }
        CommandType::Query => {}
    }
    Ok(())
}

// ============================================================
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ===== parse_command 精确匹配测试 =====

    #[test]
    fn test_parse_turn_on_light() {
        let cases = &["开灯", "打开灯", "把灯打开", "灯打开", "开灯吧"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Light, "命令 '{}' 目标应为灯", cmd);
            assert_eq!(result.cmd_type, CommandType::TurnOn, "命令 '{}' 类型应为 TurnOn", cmd);
            assert!(result.value.is_none(), "命令 '{}' 不应有数值", cmd);
        }
    }

    #[test]
    fn test_parse_turn_off_light() {
        let cases = &["关灯", "把灯关上", "灯关闭", "关灯吧"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Light);
            assert_eq!(result.cmd_type, CommandType::TurnOff);
        }
    }

    #[test]
    fn test_parse_turn_on_ac() {
        let cases = &["开空调", "打开空调", "把空调打开"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::AirCondition);
            assert_eq!(result.cmd_type, CommandType::TurnOn);
        }
    }

    #[test]
    fn test_parse_turn_off_ac() {
        let cases = &["关空调", "把空调关上"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::AirCondition);
            assert_eq!(result.cmd_type, CommandType::TurnOff);
        }
    }

    #[test]
    fn test_parse_turn_on_fan() {
        let cases = &["开风扇", "打开风扇", "把风扇打开"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Fan);
            assert_eq!(result.cmd_type, CommandType::TurnOn);
        }
    }

    #[test]
    fn test_parse_turn_off_fan() {
        let cases = &["关风扇", "把风扇关上"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Fan);
            assert_eq!(result.cmd_type, CommandType::TurnOff);
        }
    }

    #[test]
    fn test_parse_all_devices() {
        let on_cases = &["全部打开", "全部开启", "打开所有"];
        for cmd in on_cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::All);
            assert_eq!(result.cmd_type, CommandType::TurnOn);
        }

        let off_cases = &["全部关闭", "关闭所有"];
        for cmd in off_cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::All);
            assert_eq!(result.cmd_type, CommandType::TurnOff);
        }
    }

    // ===== parse_command 数值提取测试 =====

    #[test]
    fn test_parse_set_ac_temp() {
        let cases: &[(&str, u8)] = &[
            ("空调调到25度", 25),
            ("空调温度26度", 26),
            ("把空调调到20度", 20),
            ("温度调到28度", 28),
        ];
        for (cmd, expected_temp) in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::AirCondition, "命令 '{}' 目标应为空调", cmd);
            assert_eq!(result.cmd_type, CommandType::SetValue, "命令 '{}' 类型应为 SetValue", cmd);
            assert_eq!(result.value, Some(*expected_temp), "命令 '{}' 温度应为 {}", cmd, expected_temp);
        }
    }

    #[test]
    fn test_parse_set_light_brightness() {
        let cases: &[(&str, u8)] = &[
            ("灯亮度调到80", 80),
            ("灯光50", 50),
        ];
        for (cmd, expected_brightness) in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Light);
            assert_eq!(result.cmd_type, CommandType::SetValue);
            assert_eq!(result.value, Some(*expected_brightness));
        }
    }

    #[test]
    fn test_parse_set_fan_speed() {
        let cases: &[(&str, u8)] = &[
            ("风扇1档", 1),
            ("风扇2档", 2),
            ("风扇3档", 3),
        ];
        for (cmd, expected_speed) in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Fan);
            assert_eq!(result.cmd_type, CommandType::SetValue);
            assert_eq!(result.value, Some(*expected_speed));
        }
    }

    // ===== execute_command 状态更新测试 =====

    #[test]
    fn test_execute_turn_on_light() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::TurnOn,
            target: TargetDevice::Light,
            value: None,
            room: TargetRoom::LivingRoom,
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(guard.living_room.light.is_on);
    }

    #[test]
    fn test_execute_turn_off_light() {
        let state = GlobalState::new();
        {
            let mut guard = state.write().expect("获取写锁失败");
            guard.living_room.light.is_on = true;
        }
        let cmd = ParsedCommand {
            cmd_type: CommandType::TurnOff,
            target: TargetDevice::Light,
            value: None,
            room: TargetRoom::LivingRoom,
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(!guard.living_room.light.is_on);
    }

    #[test]
    fn test_execute_set_ac_temp() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::SetValue,
            target: TargetDevice::AirCondition,
            value: Some(25),
            room: TargetRoom::LivingRoom,
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert_eq!(guard.living_room.air_condition.temperature, 25);
    }

    #[test]
    fn test_execute_invalid_temp_rejected() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::SetValue,
            target: TargetDevice::AirCondition,
            value: Some(35), // 超出范围 16-30
            room: TargetRoom::LivingRoom,
        };
        let result = execute_command(&cmd, &state);
        assert!(result.is_err(), "35度应该被拒绝");
        if let Err(ControlError::InvalidValue { expected, got }) = result {
            assert_eq!(expected, "16-30");
            assert_eq!(got, 35);
        } else {
            panic!("期望 InvalidValue 错误");
        }
    }

    #[test]
    fn test_execute_invalid_brightness_rejected() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::SetValue,
            target: TargetDevice::Light,
            value: Some(150), // 超出范围 0-100
            room: TargetRoom::LivingRoom,
        };
        let result = execute_command(&cmd, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_invalid_fan_speed_rejected() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::SetValue,
            target: TargetDevice::Fan,
            value: Some(5), // 超出范围 0-3
            room: TargetRoom::LivingRoom,
        };
        let result = execute_command(&cmd, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_all_devices() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::TurnOn,
            target: TargetDevice::All,
            value: None,
            room: TargetRoom::LivingRoom,
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(guard.living_room.light.is_on);
        assert!(guard.living_room.air_condition.is_on);
        assert!(guard.living_room.fan.is_on);
    }

    #[test]
    fn test_execute_bedroom_light() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::TurnOn,
            target: TargetDevice::Light,
            value: None,
            room: TargetRoom::Bedroom,
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        // 客厅灯应该不受影响
        assert!(!guard.living_room.light.is_on);
        // 卧室灯应该开启
        assert!(guard.bedroom.light.is_on);
    }

    #[test]
    fn test_execute_bedroom_ac() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::TurnOn,
            target: TargetDevice::AirCondition,
            value: None,
            room: TargetRoom::Bedroom,
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(!guard.living_room.air_condition.is_on);
        assert!(guard.bedroom.air_condition.is_on);
    }

    #[test]
    fn test_parse_bedroom_light() {
        let cases = &["卧室开灯", "卧室灯打开", "打开卧室灯"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Light, "命令 '{}' 目标应为灯", cmd);
            assert_eq!(result.cmd_type, CommandType::TurnOn, "命令 '{}' 类型应为 TurnOn", cmd);
            assert_eq!(result.room, TargetRoom::Bedroom, "命令 '{}' 目标房间应为卧室", cmd);
        }
    }

    #[test]
    fn test_parse_living_room_light() {
        let cases = &["客厅开灯", "客厅灯打开", "打开客厅灯"];
        for cmd in cases {
            let result = parse_command(cmd).expect("解析失败");
            assert_eq!(result.target, TargetDevice::Light, "命令 '{}' 目标应为灯", cmd);
            assert_eq!(result.cmd_type, CommandType::TurnOn, "命令 '{}' 类型应为 TurnOn", cmd);
            assert_eq!(result.room, TargetRoom::LivingRoom, "命令 '{}' 目标房间应为客厅", cmd);
        }
    }

    // ===== 端到端测试 =====

    #[test]
    fn test_full_flow_turn_on_light() {
        let state = GlobalState::new();
        {
            let guard = state.read().expect("获取读锁失败");
            assert!(!guard.living_room.light.is_on);
        }

        let cmd_text = "打开灯";
        let parsed = parse_command(cmd_text).expect("解析失败");
        assert_eq!(parsed.target, TargetDevice::Light);
        assert_eq!(parsed.cmd_type, CommandType::TurnOn);

        execute_command(&parsed, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(guard.living_room.light.is_on);
    }

    #[test]
    fn test_full_flow_set_ac_temperature() {
        let state = GlobalState::new();
        {
            let guard = state.read().expect("获取读锁失败");
            assert_eq!(guard.living_room.air_condition.temperature, 24); // 默认值
        }

        let cmd_text = "空调调到26度";
        let parsed = parse_command(cmd_text).expect("解析失败");
        assert_eq!(parsed.target, TargetDevice::AirCondition);
        assert_eq!(parsed.cmd_type, CommandType::SetValue);
        assert_eq!(parsed.value, Some(26));

        execute_command(&parsed, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert_eq!(guard.living_room.air_condition.temperature, 26);
    }

    #[test]
    fn test_unrecognized_command() {
        let result = parse_command("这是乱七八糟的指令xyz");
        assert!(result.is_err());
        if let Err(ControlError::Unrecognized(s)) = result {
            assert!(s.contains("乱七八糟"));
        } else {
            panic!("期望 Unrecognized 错误");
        }
    }

    // ===== VLM JSON 解析测试 (C1) =====

    #[test]
    fn test_parse_vlm_response_turn_on_light() {
        // 新格式: {"command": {"device": "light", "action": "on", ...}, "raw_text": "..."}
        let json = r#"{"command": {"device": "light", "action": "on"}, "raw_text": "打开灯"}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.device, TargetDevice::Light);
        assert_eq!(cmd.action, CommandType::TurnOn);
    }

    #[test]
    fn test_parse_vlm_response_set_ac_temperature() {
        // 新格式带 target_value
        let json = r#"{"command": {"device": "air_condition", "action": "set_level", "target_value": 25}}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.device, TargetDevice::AirCondition);
        assert_eq!(cmd.action, CommandType::SetValue);
        assert_eq!(cmd.value, Some(25));
    }

    #[test]
    fn test_parse_vlm_response_with_confidence() {
        let json = r#"{"command": {"device": "fan", "action": "on", "confidence": 0.95}}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.device, TargetDevice::Fan);
        assert_eq!(cmd.action, CommandType::TurnOn);
    }

    #[test]
    fn test_parse_vlm_response_low_confidence() {
        let json = r#"{"command": {"device": "light", "action": "on", "confidence": 0.4}}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VoiceError::VlmLowConfidence(c) if c == 0.4));
    }

    #[test]
    fn test_parse_vlm_response_invalid_action() {
        let json = r#"{"command": {"device": "light", "action": "invalid_action"}}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VoiceError::VlmParseError(_)));
    }

    #[test]
    fn test_parse_vlm_response_invalid_device() {
        let json = r#"{"command": {"device": "unknown_device", "action": "on"}}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VoiceError::VlmParseError(_)));
    }

    #[test]
    fn test_parse_vlm_response_invalid_json() {
        let json = r#"not valid json"#;
        let result = parse_vlm_response(json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VoiceError::VlmParseError(_)));
    }

    #[test]
    fn test_parse_vlm_response_curtain_open() {
        let json = r#"{"command": {"device": "curtain", "action": "open"}}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.device, TargetDevice::Curtain);
        assert_eq!(cmd.action, CommandType::TurnOn); // open → TurnOn
    }

    #[test]
    fn test_parse_vlm_response_curtain_close() {
        let json = r#"{"command": {"device": "curtain", "action": "close"}}"#;
        let result = parse_vlm_response(json);
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.device, TargetDevice::Curtain);
        assert_eq!(cmd.action, CommandType::TurnOff); // close → TurnOff
    }

    // ===== 模糊匹配测试 (C3) =====

    #[test]
    fn test_fuzzy_match_exact() {
        // 精确匹配
        let result = fuzzy_match_command("打开灯");
        assert!(result.is_some());
        let (action, target, _) = result.unwrap();
        assert_eq!(action, CommandType::TurnOn);
        assert_eq!(target, TargetDevice::Light);
    }

    #[test]
    fn test_fuzzy_match_typo() {
        // 轻微拼写错误
        let result = fuzzy_match_command("打开灯灯");  // 多了一个"灯"
        assert!(result.is_some());
        let (action, target, _) = result.unwrap();
        assert_eq!(action, CommandType::TurnOn);
        assert_eq!(target, TargetDevice::Light);
    }

    #[test]
    fn test_fuzzy_match_short() {
        // 短输入
        let result = fuzzy_match_command("开灯");
        assert!(result.is_some());
        let (action, target, _) = result.unwrap();
        assert_eq!(action, CommandType::TurnOn);
        assert_eq!(target, TargetDevice::Light);
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        // 无法匹配
        let result = fuzzy_match_command("这是乱码指令xyz123");
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_match_ac() {
        let cases = &["开空调", "关空调", "打开空调"];
        for cmd in cases {
            let result = fuzzy_match_command(cmd);
            assert!(result.is_some(), "命令 '{}' 应能匹配", cmd);
        }
    }

    #[test]
    fn test_fuzzy_match_fan() {
        let cases = &["开风扇", "关风扇", "打开风扇"];
        for cmd in cases {
            let result = fuzzy_match_command(cmd);
            assert!(result.is_some(), "命令 '{}' 应能匹配", cmd);
        }
    }

    // ===== VoiceError Display 测试 (C4) =====

    #[test]
    fn test_voice_error_display() {
        let err = VoiceError::VlmTimeout;
        assert!(err.to_string().contains("VLM 请求超时"));

        let err = VoiceError::VlmLowConfidence(0.5);
        assert!(err.to_string().contains("0.50"));

        let err = VoiceError::UnknownCommand;
        assert!(err.to_string().contains("无法识别指令"));
    }
}