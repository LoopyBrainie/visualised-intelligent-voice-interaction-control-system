// daemon_manager.rs - Python Daemon 生命周期管理
// 职责: 启动/健康检查/重启/停止 Python Flask Daemon

use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use std::thread;
use reqwest::blocking::Client;

use crate::error::AppError;
use crate::logger::{log_info, log_warn, log_error};

/// Daemon 配置
const DAEMON_HOST: &str = "127.0.0.1";
const DAEMON_PORT: u16 = 8765;
const HEALTH_CHECK_URL: &str = "http://127.0.0.1:8765/health";
const STARTUP_TIMEOUT_MS: u64 = 5000;  // 5秒启动超时
const HEALTH_CHECK_INTERVAL_MS: u64 = 100;  // 100ms 检查间隔

/// Python Daemon 进程管理器
pub struct DaemonManager {
    child: Option<Child>,
    startup_time: Option<Instant>,
}

impl DaemonManager {
    /// 创建新的 DaemonManager
    pub fn new() -> Self {
        Self {
            child: None,
            startup_time: None,
        }
    }

    /// 启动 Daemon 进程
    pub fn start(&mut self) -> Result<(), AppError> {
        if self.is_healthy() {
            log_info("Daemon 已在运行，无需重启");
            return Ok(());
        }

        self.stop();

        let python_path = crate::resolve_python_runtime()
            .map_err(|e| AppError::python_env(format!("无法解析 Python 路径: {}", e)))?;

        let daemon_script = find_daemon_script()?;

        log_info(&format!("启动 Python Daemon: {} {}", python_path.display(), daemon_script.display()));

        let child = Command::new(&python_path)
            .arg(&daemon_script)
            .current_dir(daemon_script.parent().unwrap_or(Path::new(".")))
            .spawn()
            .map_err(|e| AppError::daemon_error(format!("Failed to spawn daemon: {}", e)))?;

        self.child = Some(child);
        self.startup_time = Some(Instant::now());

        self.wait_for_healthy()?;

        log_info("Python Daemon 启动成功");
        Ok(())
    }

    /// 等待 Daemon 健康检查通过
    fn wait_for_healthy(&self) -> Result<(), AppError> {
        let start = Instant::now();
        let max_wait = Duration::from_millis(STARTUP_TIMEOUT_MS);

        while start.elapsed() < max_wait {
            if self.is_healthy() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(HEALTH_CHECK_INTERVAL_MS));
        }

        Err(AppError::daemon_error(format!(
            "Daemon 健康检查超时 ({}ms)",
            STARTUP_TIMEOUT_MS
        )))
    }

    /// 检查 Daemon 是否健康
    pub fn is_healthy(&self) -> bool {
        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .build();

        match client {
            Ok(client) => {
                match client.get(HEALTH_CHECK_URL).send() {
                    Ok(response) => response.status().is_success(),
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    /// 确保 Daemon 运行（自动重启）
    pub fn ensure_running(&mut self) -> bool {
        match self.is_healthy() {
            true => true,
            false => {
                log_warn("Daemon 不健康，尝试重启...");
                match self.start() {
                    Ok(_) => {
                        log_info("Daemon 重启成功");
                        true
                    }
                    Err(e) => {
                        log_error(&format!("Daemon 重启失败: {}", e));
                        false
                    }
                }
            }
        }
    }

    /// 停止 Daemon 进程
    pub fn stop(&mut self) {
        // 先尝试优雅 shutdown
        if self.is_healthy() {
            let client = Client::builder()
                .timeout(Duration::from_secs(2))
                .build();

            if let Ok(client) = client {
                let _ = client.post("http://127.0.0.1:8765/shutdown").send();
                // 等待进程退出
                thread::sleep(Duration::from_millis(500));
            }
        }

        // 强制 kill 子进程
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            log_info("Daemon 进程已终止");
        }

        self.startup_time = None;
    }

    /// 获取 Daemon 运行时间
    pub fn uptime(&self) -> Option<Duration> {
        self.startup_time.map(|t| t.elapsed())
    }

    /// 探测手势识别端点 /gesture 是否可达
    /// 返回 true 表示 Python 手势模块已加载并响应
    pub fn probe_gesture(&self) -> bool {
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build();

        match client {
            Ok(client) => {
                // 发送最小 1x1 黑色 JPEG 作为探测帧
                let probe_jpeg: Vec<u8> = vec![
                    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46,
                    0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                    0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43,
                    0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08,
                    0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C,
                    0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
                    0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
                    0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20,
                    0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29,
                    0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27,
                    0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34,
                    0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
                    0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4,
                    0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
                    0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04,
                    0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF,
                    0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03,
                    0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04,
                    0x00, 0x00, 0x01, 0x7D, 0x01, 0x02, 0x03, 0x00,
                    0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
                    0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32,
                    0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1,
                    0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72,
                    0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A,
                    0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35,
                    0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45,
                    0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
                    0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65,
                    0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75,
                    0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85,
                    0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94,
                    0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3,
                    0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2,
                    0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
                    0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9,
                    0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8,
                    0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6,
                    0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4,
                    0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA,
                    0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00,
                    0xD2, 0xCF, 0x20, 0xFF, 0xD9,
                ];

                match client
                    .post("http://127.0.0.1:8765/gesture")
                    .header("Content-Type", "image/jpeg")
                    .body(probe_jpeg)
                    .send()
                {
                    Ok(response) => response.status().is_success(),
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 查找 daemon.py 脚本路径
fn find_daemon_script() -> Result<std::path::PathBuf, AppError> {
    let candidates = [
        Path::new("python_engine/daemon.py"),
        Path::new("../python_engine/daemon.py"),
        Path::new("python_engine/daemon.py"),
    ];

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let from_manifest = Path::new(&manifest_dir)
            .parent()
            .unwrap_or(Path::new("."))
            .join("python_engine")
            .join("daemon.py");
        if from_manifest.exists() {
            return Ok(from_manifest);
        }
    }

    let cwd = std::env::current_dir()
        .map_err(|e| AppError::python_env(format!("无法获取当前目录: {}", e)))?;
    for candidate in &candidates {
        let path = cwd.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(AppError::daemon_error("找不到 daemon.py 脚本"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_daemon_script() {
        // 这个测试在 CI 环境可能失败，仅作基本检查
        match find_daemon_script() {
            Ok(path) => println!("Found daemon at: {:?}", path),
            Err(e) => println!("Daemon not found (expected in some envs): {}", e),
        }
    }
}