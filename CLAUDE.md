# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 构建与运行命令

```bash
# 前端
bun run dev        # Vite dev server
bun run build

# Tauri (完整应用)
bunx tauri dev
bunx tauri build

# Rust 后端 (必须用 uv 确保 PyO3 链接正确 Python)
cd src-tauri && uv run --project ../python_engine cargo build
cd src-tauri && uv run --project ../python_engine cargo check
cd src-tauri && uv run --project ../python_engine cargo test --lib
```

## PyO3 环境要求

直接运行 `cargo build` 会因 PyO3 找不到正确 Python 版本而失败。**必须使用 `uv run --project ../python_engine`** 确保使用 `python_engine/.venv` 中的 Python。

## 架构

- **GUI**: SolidJS (TSX) + Tailwind + Canvas
- **Logic**: Tauri + PyO3 + Tokio
- **Data**: NumPy + SciPy + MediaPipe

PyO3 通过 `build.rs` 动态检测 uv 虚拟环境路径。

## 关键模块

| 文件 | 职责 |
|------|------|
| `src-tauri/src/state.rs` | 设备状态 (`DeviceState`, `GlobalState`) |
| `src-tauri/src/control.rs` | 指令解析 |
| `src-tauri/src/voice.rs` | cpal 音频采集 + FFT |
| `src-tauri/src/py_engine.rs` | PyO3 绑定 |
| `src-tauri/src/logger.rs` | 日志重定向到前端 |

## Tauri Commands

| 命令 | 作用 |
|------|------|
| `handle_voice_command` | 解析并执行语音指令 |
| `get_device_state` | 获取设备状态 |
| `start_voice_capture` / `stop_voice_capture` | 语音采集控制 |

事件: `device-state-changed` (后端→前端状态同步)
