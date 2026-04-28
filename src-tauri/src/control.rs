// control.rs - 指令解析与设备控制模块
// 职责: 指令字典、指令解析、设备状态更新
use crate::state::{AirConditionState, GlobalState, LightState, FanState};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    All,
}

/// 解析后的指令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub cmd_type: CommandType,
    pub target: TargetDevice,
    pub value: Option<u8>, // 用于 SetValue (温度/亮度/风速)
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
/// 2. 精确匹配: 查找 COMMAND_DICT
/// 3. 正则提取: 动态数值（如温度、亮度）
/// 4. 模糊匹配 fallback: 通过 PyO3 调用 Python rapidfuzz
///
/// # Arguments
/// * `input` - 原始语音指令字符串
///
/// # Returns
/// * `Ok(ParsedCommand)` - 解析成功
/// * `Err(ControlError)` - 解析失败
pub fn parse_command(input: &str) -> Result<ParsedCommand, ControlError> {
    let normalized = input.trim().to_lowercase();

    // Step 1: 精确匹配
    if let Some(entry) = COMMAND_DICT.get(normalized.as_str()) {
        return Ok(ParsedCommand {
            cmd_type: entry.0,
            target: entry.1,
            value: entry.2,
        });
    }

    // Step 2: 正则提取数值
    for (pattern, target) in VALUE_PATTERNS.iter() {
        if let Some(caps) = Regex::new(pattern)
            .ok()
            .and_then(|r| r.captures(&normalized))
        {
            if let Some(val_str) = caps.get(1) {
                if let Ok(val) = val_str.as_str().parse::<u8>() {
                    return Ok(ParsedCommand {
                        cmd_type: CommandType::SetValue,
                        target: *target,
                        value: Some(val),
                    });
                }
            }
        }
    }

    // Step 3: PyO3 模糊匹配 fallback
    match py_fuzzy_match(&normalized) {
        Ok(cmd) => Ok(cmd),
        Err(_) => Err(ControlError::Unrecognized(input.to_string())),
    }
}

/// 通过 PyO3 调用 Python rapidfuzz 进行模糊匹配
///
/// # Arguments
/// * `input` - 标准化后的指令字符串
///
/// # Returns
/// * `Ok(ParsedCommand)` - 模糊匹配成功
/// * `Err(ControlError::Unrecognized)` - 匹配失败
///
/// # TODO (模块4)
/// 需要实现 PyO3 调用 python_engine 中的 rapidfuzz 封装:
/// 1. 在 py_engine.rs 中添加 Python 模糊匹配函数
/// 2. 传入指令模板列表（如 COMMAND_DICT 的 key）
/// 3. 使用 rapidfuzz.fuzz.ratio() 计算相似度
/// 4. 阈值 > 80% 时返回最佳匹配结果
fn py_fuzzy_match(input: &str) -> Result<ParsedCommand, ControlError> {
    // TODO (模块4): 实现 PyO3 rapidfuzz 集成
    // 临时返回错误，让未知指令走到 Unrecognized
    // 预期实现:
    //   let best_match = py_fuzzy_match_internal(input, &COMMAND_DICT_KEYS)?;
    //   if best_match.score > 80 { ... }
    Err(ControlError::Unrecognized(input.to_string()))
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

    match cmd.target {
        TargetDevice::Light => apply_light(cmd, &mut guard.light),
        TargetDevice::AirCondition => apply_ac(cmd, &mut guard.air_condition),
        TargetDevice::Fan => apply_fan(cmd, &mut guard.fan),
        TargetDevice::All => {
            apply_light(cmd, &mut guard.light)?;
            apply_ac(cmd, &mut guard.air_condition)?;
            apply_fan(cmd, &mut guard.fan)?;
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
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(guard.light.is_on);
    }

    #[test]
    fn test_execute_turn_off_light() {
        let state = GlobalState::new();
        {
            let mut guard = state.write().expect("获取写锁失败");
            guard.light.is_on = true;
        }
        let cmd = ParsedCommand {
            cmd_type: CommandType::TurnOff,
            target: TargetDevice::Light,
            value: None,
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(!guard.light.is_on);
    }

    #[test]
    fn test_execute_set_ac_temp() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::SetValue,
            target: TargetDevice::AirCondition,
            value: Some(25),
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert_eq!(guard.air_condition.temperature, 25);
    }

    #[test]
    fn test_execute_invalid_temp_rejected() {
        let state = GlobalState::new();
        let cmd = ParsedCommand {
            cmd_type: CommandType::SetValue,
            target: TargetDevice::AirCondition,
            value: Some(35), // 超出范围 16-30
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
        };
        execute_command(&cmd, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(guard.light.is_on);
        assert!(guard.air_condition.is_on);
        assert!(guard.fan.is_on);
    }

    // ===== 端到端测试 =====

    #[test]
    fn test_full_flow_turn_on_light() {
        let state = GlobalState::new();
        {
            let guard = state.read().expect("获取读锁失败");
            assert!(!guard.light.is_on);
        }

        let cmd_text = "打开灯";
        let parsed = parse_command(cmd_text).expect("解析失败");
        assert_eq!(parsed.target, TargetDevice::Light);
        assert_eq!(parsed.cmd_type, CommandType::TurnOn);

        execute_command(&parsed, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert!(guard.light.is_on);
    }

    #[test]
    fn test_full_flow_set_ac_temperature() {
        let state = GlobalState::new();
        {
            let guard = state.read().expect("获取读锁失败");
            assert_eq!(guard.air_condition.temperature, 24); // 默认值
        }

        let cmd_text = "空调调到26度";
        let parsed = parse_command(cmd_text).expect("解析失败");
        assert_eq!(parsed.target, TargetDevice::AirCondition);
        assert_eq!(parsed.cmd_type, CommandType::SetValue);
        assert_eq!(parsed.value, Some(26));

        execute_command(&parsed, &state).expect("执行失败");
        let guard = state.read().expect("获取读锁失败");
        assert_eq!(guard.air_condition.temperature, 26);
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
}