# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

可视化智能语音交互控制系统 - 一个智能家居仿真项目，使用 **Tauri (Rust) + SolidJS + Python** 技术栈实现语音/手势双模式控制。

## 构建与运行命令

```bash
# 前端开发
bun run dev        # Vite dev server (端口 1420)
bun run build      # 生产构建

# Tauri 应用 (完整应用，含 Rust 后端)
bunx tauri dev     # 开发模式运行完整应用
bunx tauri build   # 生产构建

# Rust 后端
cd src-tauri && uv run --project ../python_engine cargo build   # 编译 Rust
cd src-tauri && uv run --project ../python_engine cargo check   # 类型检查
```

## PyO3 兼容性说明

PyO3 需要正确链接 Python 解释器。使用 `uv run` 确保在正确的虚拟环境中执行：

```bash
# Rust 编译（使用 python_engine/.venv 中的 Python）
uv run --project ../python_engine cargo build

# Tauri 开发模式
uv run --project python_engine bun tauri dev

# 或在 python_engine 目录下直接执行
cd python_engine && uv run cargo build
```

> **注意**: 直接运行 `cargo build` 可能导致 PyO3 找不到正确的 Python 版本，因为系统 Python 与 `.venv` 中的版本可能不兼容。

## 架构概览

项目采用 Tauri + SolidJS + Python (PyO3) 三层架构：
- **GUI**: SolidJS (TSX) + Tailwind + Canvas
- **Logic**: Tauri + PyO3 + Tokio
- **Data**: NumPy + SciPy + MediaPipe

PyO3 配置通过 `build.rs` 动态检测 uv 虚拟环境路径，兼容 Windows (Scripts) 和 Unix (bin)。应用启动时自动调用 `uv sync` 初始化环境（若未就绪）。

## 重要参考文档

- **[PROJECT_PLAN.md](PROJECT_PLAN.md)** - 主要架构文档，包含完整模块分解、验收标准、算法说明、8周实施计划
- **[实验要求.md](实验要求.md)** - 实验任务书，定义5个核心模块功能要求
