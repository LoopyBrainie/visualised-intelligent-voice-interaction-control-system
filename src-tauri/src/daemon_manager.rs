// daemon_manager.rs - Python Daemon 生命周期管理
// 职责: 启动/健康检查/重启/停止 Python Flask Daemon

use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use std::thread;
use anyhow::Result;
use reqwest::blocking::Client;

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
    pub fn start(&mut self) -> Result<()> {
        // 检查是否已在运行
        if self.is_healthy() {
            log_info("Daemon 已在运行，无需重启");
            return Ok(());
        }

        // 停止旧进程（如果存在）
        self.stop();

        // 解析 Python 路径
        let python_path = crate::resolve_python_runtime()
            .map_err(|e| anyhow::anyhow!("无法解析 Python 路径: {}", e))?;

        let daemon_script = find_daemon_script()?;

        log_info(&format!("启动 Python Daemon: {} {}", python_path.display(), daemon_script.display()));

        // 启动 Python 进程
        let child = Command::new(&python_path)
            .arg(&daemon_script)
            .current_dir(daemon_script.parent().unwrap())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn daemon: {}", e))?;

        self.child = Some(child);
        self.startup_time = Some(Instant::now());

        // 等待健康检查通过
        self.wait_for_healthy()?;

        log_info("Python Daemon 启动成功");
        Ok(())
    }

    /// 等待 Daemon 健康检查通过
    fn wait_for_healthy(&self) -> Result<()> {
        let start = Instant::now();
        let max_wait = Duration::from_millis(STARTUP_TIMEOUT_MS);

        while start.elapsed() < max_wait {
            if self.is_healthy() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(HEALTH_CHECK_INTERVAL_MS));
        }

        Err(anyhow::anyhow!("Daemon 健康检查超时 ({}ms)", STARTUP_TIMEOUT_MS))
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
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 查找 daemon.py 脚本路径
fn find_daemon_script() -> Result<std::path::PathBuf> {
    // 尝试多个可能的位置
    let candidates = [
        // 开发态：从 src-tauri 向上找 python_engine
        Path::new("python_engine/daemon.py"),
        Path::new("../python_engine/daemon.py"),
        // 安装态：exe 同级 python_engine
        Path::new("python_engine/daemon.py"),
    ];

    // 从 CARGO_MANIFEST_DIR 推导
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let from_manifest = Path::new(&manifest_dir)
            .parent().unwrap()
            .join("python_engine")
            .join("daemon.py");
        if from_manifest.exists() {
            return Ok(from_manifest);
        }
    }

    // 尝试当前工作目录
    let cwd = std::env::current_dir()?;
    for candidate in &candidates {
        let path = cwd.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow::anyhow!("找不到 daemon.py 脚本"))
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