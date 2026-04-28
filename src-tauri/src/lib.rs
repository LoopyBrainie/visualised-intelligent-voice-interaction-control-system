// lib.rs - 模块导出
// 模块1: voice.rs (语音采集), logger.rs (日志)
// 模块2: state.rs (状态), control.rs (指令控制)
// 模块4: py_engine.rs (PyO3绑定)

mod logger;
mod py_engine;
mod state;
mod control;

use std::env;
use std::path::PathBuf;
use std::process::Command;
use anyhow::Result;
use tauri::{Emitter, Manager};
use logger::{log_error, log_info, log_warn};
use state::GlobalState;
use control::{parse_command, execute_command};

/// 核心逻辑：寻找并同步 uv 环境，返回 Python 解释器路径
fn resolve_python_runtime() -> anyhow::Result<PathBuf> {
    // 1. 获取基准目录（支持开发态和安装态）
    // 生产环境下通常是安装目录，Windows 结构: bin/ -> 上级目录
    let base_dir = env::current_exe()?
        .parent().unwrap()   // bin 目录
        .parent().unwrap()   // 根目录
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
                .parent().unwrap()
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
    let _ = app.emit("env-error", err);
}

/// 获取运行时 site-packages 目录（fallback 备用）
fn get_runtime_libs_dir() -> PathBuf {
    let exe_dir = env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap()
        .to_path_buf();

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
        let manifest = PathBuf::from(&manifest_dir).parent().unwrap()
            .join("python_engine").join(".venv").join("Lib").join("site-packages");
        if manifest.exists() {
            return manifest;
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

/// 处理语音指令的 Tauri command
#[tauri::command]
fn handle_voice_command(cmd: &str, state: tauri::State<GlobalState>, app_handle: tauri::AppHandle) -> Result<String, String> {
    // 1. 解析指令
    let parsed = parse_command(cmd).map_err(|e| format!("{:?}", e))?;

    // 2. 执行指令（直接操作全局状态）
    execute_command(&parsed, &state).map_err(|e| format!("{:?}", e))?;

    // 3. 获取更新后的状态并广播到前端
    let device_state = {
        let guard = state.read().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    // 4. 广播状态更新到前端
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.emit(EMIT_DEVICE_UPDATE, &device_state);
    }

    Ok(format!("{:?}", parsed))
}

/// 获取当前设备状态
#[tauri::command]
fn get_device_state(state: tauri::State<GlobalState>) -> Result<String, String> {
    let guard = state.read().map_err(|e| format!("Lock error: {}", e))?;
    serde_json::to_string(&*guard).map_err(|e| format!("Serialization error: {}", e))
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
        .invoke_handler(tauri::generate_handler![greet, handle_voice_command, get_device_state, start_voice_capture, stop_voice_capture])
        .setup(|app| {
            // C. 初始化日志系统
            if let Err(e) = logger::init_logger(app.handle().clone()) {
                eprintln!("Failed to init logger: {}", e);
            }
            log_info("日志系统初始化完成");
            log_warn("这是一条警告测试日志");
            log_error("这是一条错误测试日志");

            // D. 初始化 Python 环境 (auto-initialize 自动处理)
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

/// 开始语音采集
#[tauri::command]
fn start_voice_capture() -> Result<String, String> {
    log_info("语音采集开始");
    // TODO: 后续对接 voice.rs 中的 cpal 录音逻辑
    Ok("started".to_string())
}

/// 停止语音采集
#[tauri::command]
fn stop_voice_capture() -> Result<String, String> {
    log_info("语音采集停止");
    // TODO: 后续对接 voice.rs 中的 cpal 停止逻辑
    Ok("stopped".to_string())
}
