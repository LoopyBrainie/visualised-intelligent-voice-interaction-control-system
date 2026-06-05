// lib.rs - 模块导出
// 模块1: voice.rs (语音采集), logger.rs (日志)
// 模块2: state.rs (状态), control.rs (指令控制)
// 模块4: py_engine.rs (PyO3绑定)

mod error;
mod logger;
mod py_engine;
mod state;
mod control;
mod voice;
mod daemon_manager;
mod gesture;

use std::env;
use std::path::PathBuf;
use std::process::Command;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use anyhow::Result;
use tauri::{Emitter, Manager};
use crate::error::AppError;
use logger::{log_error, log_info, log_warn};
use state::{GlobalState, DaemonState};
use control::{parse_command, execute_command};

/// 核心逻辑：寻找并同步 uv 环境，返回 Python 解释器路径
fn resolve_python_runtime() -> anyhow::Result<PathBuf> {
    // 1. 获取基准目录（支持开发态和安装态）
    // 生产环境下通常是安装目录，Windows 结构: bin/ -> 上级目录
    let base_dir = env::current_exe()?
        .parent().ok_or_else(|| anyhow::anyhow!("无法获取 exe 父目录"))?
        .parent().ok_or_else(|| anyhow::anyhow!("无法获取安装根目录"))?
        .to_path_buf();

    // 2. 针对不同环境的 python_engine 路径探测
    let engine_dir = if base_dir.join("python_engine").exists() {
        base_dir.join("python_engine")
    } else {
        // 开发态回退：从 src-tauri 回退到根目录
        // 先尝试项目根目录
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        if !manifest_dir.is_empty() {
            PathBuf::from(&manifest_dir)
                .parent().ok_or_else(|| anyhow::anyhow!("无法解析 CARGO_MANIFEST_DIR 父目录"))?
                .join("python_engine")
        } else {
            // 最终回退到当前工作目录
            env::current_dir()?.join("python_engine")
        }
    };

    let venv_dir = engine_dir.join(".venv");

    // 3. 触发 uv sync (若 venv 不存在则自动创建)
    if !venv_dir.exists() {
        println!("[Python Runtime] 检测到环境缺失，正在调用系统 uv 同步环境...");
        Command::new("uv")
            .arg("sync")
            .current_dir(&engine_dir)
            .status()
            .map_err(|_| anyhow::anyhow!("请确保系统已安装 uv 工具"))?;
    }

    // 4. 跨平台返回 Python 路径
    let python_exe = if cfg!(windows) {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    };

    Ok(python_exe)
}

/// 将错误通过 Tauri 事件发送到前端
fn emit_env_error(app: &tauri::AppHandle, err: &str) {
    // intentionally ignored: one-way UI notification, no recovery path
    let _ = app.emit("env-error", err);
}

/// 获取运行时 site-packages 目录（fallback 备用）
fn get_runtime_libs_dir() -> PathBuf {
    let exe_dir = match env::current_exe() {
        Ok(exe) => exe.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
        Err(_) => env::current_dir().unwrap_or_default(),
    };

    // 1. exe 同级 python_engine
    let sibling = exe_dir.join("python_engine").join(".venv").join("Lib").join("site-packages");
    if sibling.exists() {
        return sibling;
    }

    // 2. exe 向上两级 + python_engine
    let parent2 = exe_dir.join("..").join("..").join("python_engine").join(".venv").join("Lib").join("site-packages");
    if parent2.exists() {
        return parent2;
    }

    // 3. CARGO_MANIFEST_DIR 回退
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    if !manifest_dir.is_empty() {
        if let Some(parent) = PathBuf::from(&manifest_dir).parent() {
            let manifest = parent.join("python_engine").join(".venv").join("Lib").join("site-packages");
            if manifest.exists() {
                return manifest;
            }
        }
    }

    // 4. 最终回退到当前工作目录
    env::current_dir().unwrap_or_default()
        .join("python_engine").join(".venv").join("Lib").join("site-packages")
}

/// 通过 Python sys.base_executable 找到真正的 DLL 目录
/// sys.base_executable 指向原始 Python 安装路径，不是 venv 的 shim
fn get_base_python_dir(python_exe: &PathBuf) -> Option<PathBuf> {
    let output = Command::new(python_exe)
        .arg("-c")
        .arg("import sys; import os; print(os.path.dirname(sys.base_executable))")
        .output()
        .ok()?;

    if !output.status.success() {
        eprintln!("[DLL Path] Python 命令执行失败");
        return None;
    }

    let base_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if base_dir.is_empty() {
        eprintln!("[DLL Path] Python 返回空路径");
        return None;
    }

    let path = PathBuf::from(&base_dir);
    if path.exists() {
        println!("[DLL Path] Base Python 目录: {:?}", path);
        Some(path)
    } else {
        eprintln!("[DLL Path] Base Python 目录不存在: {}", base_dir);
        None
    }
}

/// 将 DLL 目录添加到 PATH，确保 Windows 能找到 python313.dll
fn setup_dll_path(python_exe: &PathBuf) -> Result<()> {
    // 方法1（最可靠）：让 Python 告诉我们它真正的 DLL 目录
    if let Some(base_dir) = get_base_python_dir(python_exe) {
        if let Ok(current_path) = env::var("PATH") {
            if !current_path.contains(base_dir.to_str().unwrap_or("")) {
                let mut paths: Vec<_> = std::env::split_paths(&current_path).collect();
                paths.insert(0, base_dir.clone());
                if let Ok(new_path) = std::env::join_paths(&paths) {
                    env::set_var("PATH", new_path);
                    println!("[DLL Path] 已添加 Base Python 目录到 PATH: {:?}", base_dir);
                }
            }
        }
        return Ok(());
    }

    // 方法2：回退到手动探测 .libs 目录
    let site_packages = get_runtime_libs_dir();
    for libs_name in ["numpy.libs", "scipy.libs", "sklearn.libs", "llvmlite.libs"] {
        let libs_path = site_packages.join(libs_name);
        if libs_path.exists() {
            if let Ok(current_path) = env::var("PATH") {
                if !current_path.contains(libs_name) {
                    let new_path = format!("{};{}", libs_path.display(), current_path);
                    env::set_var("PATH", new_path);
                    println!("[DLL Path] 已添加 {} 到 PATH", libs_name);
                }
            }
        }
    }

    // 方法3：保险措施 - 使用 sys.base_prefix 寻找 DLL 所在根目录
    // sys.base_prefix 指向 Python 安装根目录（区别于 venv 前缀）
    if let Ok(output) = Command::new(python_exe)
        .arg("-c")
        .arg("import sys; print(sys.base_prefix)")
        .output()
    {
        if output.status.success() {
            let base_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let base_path = PathBuf::from(&base_dir);
            if base_path.exists() {
                if let Ok(current_path) = env::var("PATH") {
                    let base_str = base_path.to_str().unwrap_or("");
                    if !current_path.contains(base_str) {
                        let mut paths: Vec<_> = std::env::split_paths(&current_path).collect();
                        paths.insert(0, base_path.clone());
                        // 额外保险：Windows 安装版 Python 的 DLL 可能在 DLLs 文件夹
                        let dlls_path = base_path.join("DLLs");
                        if dlls_path.exists() {
                            paths.insert(0, dlls_path);
                        }
                        if let Ok(new_path) = std::env::join_paths(&paths) {
                            env::set_var("PATH", new_path);
                            println!("[DLL Path] 已通过 sys.base_prefix 添加到 PATH: {:?}", base_path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// 确保 Python 环境就绪（若 venv 不存在则调用 uv sync）
fn ensure_python_env() -> Result<()> {
    let python_path = resolve_python_runtime()?;

    // 关键: 在任何 PyO3 调用前设置环境变量
    if let Some(path_str) = python_path.to_str() {
        env::set_var("PYO3_PYTHON", path_str);
        println!("[Python Runtime] PYO3_PYTHON 设置为: {}", path_str);
    }

    // 将 DLL 目录添加到 PATH，让 Windows 能找到 python313.dll
    setup_dll_path(&python_path)?;

    Ok(())
}

// Tauri 事件名称常量
const EMIT_DEVICE_UPDATE: &str = "device-state-changed";

// VoiceManager 生命周期管理（模块级别）
static VOICE_MANAGER: Lazy<Mutex<Option<voice::VoiceManager>>> = Lazy::new(|| Mutex::new(None));

// GestureManager 生命周期管理（模块级别）
static GESTURE_MANAGER: Lazy<Mutex<Option<gesture::GestureManager>>> = Lazy::new(|| Mutex::new(None));

/// 处理语音指令的 Tauri command
#[tauri::command]
fn handle_voice_command(cmd: &str, room: Option<&str>, state: tauri::State<GlobalState>, app_handle: tauri::AppHandle) -> Result<String, AppError> {
    let mut parsed = parse_command(cmd)?;

    if let Some(room_str) = room {
        parsed.room = match room_str {
            "bedroom" => control::TargetRoom::Bedroom,
            "living_room" | "livingroom" => control::TargetRoom::LivingRoom,
            _ => parsed.room,
        };
    }

    execute_command(&parsed, &state)?;

    let device_state = {
        let guard = state.read()?;
        guard.clone()
    };

    if let Some(window) = app_handle.get_webview_window("main") {
        // intentionally ignored: one-way UI notification, no recovery path
        let _ = window.emit(EMIT_DEVICE_UPDATE, &device_state);
    }

    Ok(format!("{:?}", parsed))
}

/// 获取当前设备状态
#[tauri::command]
fn get_device_state(state: tauri::State<GlobalState>) -> Result<String, AppError> {
    let guard = state.read()?;
    Ok(serde_json::to_string(&*guard)?)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // A. 最高优先级: 在所有 PyO3 调用前注入环境变量
    if let Err(e) = ensure_python_env() {
        eprintln!("[Python Runtime] 环境初始化失败: {}", e);
        // 不退出，让应用继续启动，后续会通过事件通知前端
    }

    // B. 创建全局状态
    let global_state = GlobalState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(global_state)  // 注册全局状态
        .manage(DaemonState::new())  // 注册 Daemon 进程管理
        .invoke_handler(tauri::generate_handler![greet, handle_voice_command, get_device_state, start_voice_capture, stop_voice_capture, list_audio_devices, list_cameras, set_language_mode, start_gesture_capture, stop_gesture_capture, get_gesture_health, get_camera_frame])
        .setup(|app| {
            // C. 初始化日志系统
            if let Err(e) = logger::init_logger(app.handle().clone()) {
                eprintln!("Failed to init logger: {}", e);
            }
            log_info("日志系统初始化完成");
            log_warn("这是一条警告测试日志");
            log_error("这是一条错误测试日志");

            // D. 启动 Python Daemon
            let daemon_state = app.state::<DaemonState>();
            match daemon_state.manager.lock() {
                Ok(mut manager) => {
                    match manager.start() {
                        Ok(_) => log_info("Python Daemon 启动成功"),
                        Err(e) => {
                            let err_msg = format!("Python Daemon 启动失败: {}", e);
                            log_error(&err_msg);
                            emit_env_error(app.handle(), &err_msg);
                        }
                    }
                }
                Err(e) => log_error(&format!("获取 DaemonState 锁失败: {}", e)),
            }

            // E. 初始化 Python 环境 (auto-initialize 自动处理)
            // 此时 PyO3 会读取上面 set_var 注入的路径
            match py_engine::init_python() {
                Ok(_) => {
                    log_info("Python 虚拟机初始化成功");
                }
                Err(e) => {
                    let err_msg = format!("Python 虚拟机初始化失败: {}", e);
                    log_error(&err_msg);
                    emit_env_error(app.handle(), &err_msg);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 占位函数，确保编译通过
#[tauri::command]
fn greet(name: &str) -> String {
    log_info(&format!("收到问候请求: {}", name));
    format!("Hello, {}! 待实现完整功能", name)
}

/// 开始语音采集（同时启用录制模式）
#[tauri::command]
fn start_voice_capture(app_handle: tauri::AppHandle, state: tauri::State<GlobalState>) -> Result<String, AppError> {
    log_info("语音采集开始");

    let guard = VOICE_MANAGER.lock()?;
    if let Some(ref manager) = *guard {
        // 已存在管理器，直接启用录制模式
        manager.start_recording();
        log_info("VoiceManager 复用，录制模式已启用");
        Ok("already_running".to_string())
    } else {
        drop(guard); // 释放锁，避免 clone 时借用冲突
        let state_clone = (*state).clone();
        match voice::VoiceManager::start(app_handle, state_clone) {
            Ok(manager) => {
                manager.start_recording();
                let mut guard = VOICE_MANAGER.lock()?;
                *guard = Some(manager);
                log_info("VoiceManager 启动成功，录制模式已启用");
                Ok("started".to_string())
            }
            Err(e) => {
                log_error(&format!("VoiceManager 启动失败: {}", e));
                Err(e)
            }
        }
    }
}

/// 停止语音采集（同时停止录制并发送完整音频到 VLM）
#[tauri::command]
fn stop_voice_capture() -> Result<String, AppError> {
    log_info("语音采集停止");

    let guard = VOICE_MANAGER.lock()?;
    if let Some(ref manager) = *guard {
        // 停止录制并发送完整音频到 VLM
        manager.stop_recording_and_send();
        log_info("录制停止，VLM 请求已发送");
        Ok("stopped".to_string())
    } else {
        Ok("not_running".to_string())
    }
}

/// 列出可用麦克风设备
#[tauri::command]
fn list_audio_devices() -> Result<Vec<voice::MicrophoneDevice>, AppError> {
    Ok(voice::list_devices())
}

/// 设置语言模式
/// enabled=true: 语言模式（开启 FFT 和音频采集）
/// enabled=false: 手势模式（暂停 FFT 和音频流以节省功耗）
/// 注意：如果 VoiceManager 未运行，在语言模式下会自动启动
#[tauri::command]
fn set_language_mode(
    enabled: bool,
    app_handle: tauri::AppHandle,
    state: tauri::State<GlobalState>,
) -> Result<(), AppError> {
    let mut guard = VOICE_MANAGER.lock()?;

    // 如果语言模式启用但 VoiceManager 未运行，自动启动它
    if enabled && guard.is_none() {
        let state_clone = (*state).clone();
        match voice::VoiceManager::start(app_handle, state_clone) {
            Ok(manager) => {
                // 语言模式默认开启
                manager.set_language_mode(true);
                *guard = Some(manager);
                log_info("语言模式启动: VoiceManager 已自动启动并设为语言模式");
            }
            Err(e) => {
                log_error(&format!("自动启动 VoiceManager 失败: {}", e));
                return Err(e);
            }
        }
    } else if let Some(ref manager) = *guard {
        manager.set_language_mode(enabled);
        log_info(&format!("语言模式已设置为: {}", if enabled { "开启" } else { "关闭" }));
    }

    Ok(())
}

/// 列出可用摄像头
#[tauri::command]
fn list_cameras() -> Result<Vec<gesture::CameraInfo>, AppError> {
    Ok(gesture::list_cameras())
}

/// 开始手势采集（打开摄像头 + 启动 Python 手势识别管线）
/// `camera_id`: 可选摄像头标识符，不传则使用默认摄像头（索引 0）
#[tauri::command]
fn start_gesture_capture(app_handle: tauri::AppHandle, state: tauri::State<GlobalState>, camera_id: Option<String>) -> Result<String, AppError> {
    log_info(&format!("手势采集开始, camera_id: {:?}", camera_id));

    let mut guard = GESTURE_MANAGER.lock()?;
    if let Some(ref manager) = *guard {
        if manager.is_active() {
            log_info("GestureManager 已在运行");
            return Ok("already_running".to_string());
        }
    }

    // 停止旧的，启动新的
    if let Some(ref mut manager) = *guard {
        manager.stop();
    }
    *guard = None;
    drop(guard);

    let state_clone = (*state).clone();
    match gesture::GestureManager::start(app_handle.clone(), state_clone, camera_id) {
        Ok(manager) => {
            let mut guard = GESTURE_MANAGER.lock()?;
            *guard = Some(manager);
            log_info("GestureManager 启动成功");
            Ok("started".to_string())
        }
        Err(e) => {
            log_error(&format!("GestureManager 启动失败: {}", e));
            Err(e)
        }
    }
}

/// 停止手势采集（关闭摄像头 + 停止手势识别线程）
#[tauri::command]
fn stop_gesture_capture() -> Result<String, AppError> {
    log_info("手势采集停止");

    let mut guard = GESTURE_MANAGER.lock()?;
    if let Some(ref mut manager) = *guard {
        manager.stop();
        *guard = None;
        log_info("GestureManager 已停止");
        Ok("stopped".to_string())
    } else {
        Ok("not_running".to_string())
    }
}

/// 获取手势系统健康状态
#[tauri::command]
fn get_gesture_health() -> Result<gesture::GestureHealth, AppError> {
    let guard = GESTURE_MANAGER.lock()?;
    if let Some(ref manager) = *guard {
        Ok(manager.context_snapshot().health)
    } else {
        Ok(gesture::GestureHealth::default())
    }
}

/// 获取最新一帧摄像头 JPEG 字节（供前端 data URL 渲染使用）
/// 返回 None 表示摄像头尚未产出第一帧（仍在初始化中）。
/// 设计动机: 旧版 camera:// 自定义协议在 Tauri 2 + WebView2 上
/// WebView2 自身不识别该 scheme，请求在到达 Tauri 主机进程前就被
/// webview 拒绝 (ERR_UNKNOWN_URL_SCHEME)。改用 Tauri command +
/// Blob URL 完全绕开 scheme 注册，零 WebView2 兼容性问题。
#[tauri::command]
fn get_camera_frame() -> Result<Option<Vec<u8>>, AppError> {
    let guard = GESTURE_MANAGER.lock()
        .map_err(|e| AppError::from(e))?;
    Ok(guard.as_ref().and_then(|m| m.latest_frame()))
}
