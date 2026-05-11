<div align="center">
  <img src="src-tauri/icons/icon.png" width="96" alt="SPARV logo">

  # 可视化智能语音交互控制系统

  *Voice-controlled smart home dashboard with real-time spectrum visualization*

  [![Tauri 2](https://img.shields.io/badge/Tauri-2-ffc131?style=flat-square&logo=tauri&logoColor=black)](https://tauri.app)
  [![SolidJS](https://img.shields.io/badge/SolidJS-4f88c6?style=flat-square&logo=solid&logoColor=white)](https://solidjs.com)
  [![Python](https://img.shields.io/badge/Python->=3.12-3776ab?style=flat-square&logo=python&logoColor=white)](https://python.org)
  [![Rust](https://img.shields.io/badge/Rust-dea584?style=flat-square&logo=rust&logoColor=white)](https://rust-lang.org)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)

</div>

A desktop application that lets you control smart home devices through voice commands. Speak naturally — "打开客厅灯" or "空调调到25度" — and watch the UI respond in real-time with live audio spectrum visualization.

## Features

- **Voice Command Control** — Control lights, air conditioning, fans, and curtains across multiple rooms using natural Chinese speech
- **VLM-Powered Recognition** — Audio is processed by a Vision-Language Model for high-accuracy command extraction with confidence scoring
- **Real-time Spectrum Visualization** — Live FFT audio spectrum rendered at 60Hz while voice mode is active
- **Multi-Room Support** — Independent device states for living room (客厅) and bedroom (卧室)
- **Dual-Mode Operation** — Switch between voice mode (active listening + FFT) and gesture mode (audio paused to save power)
- **Full-Chain Logging** — Traceable logs with `trace_id` flowing from Rust through Python back to the frontend

## Architecture

```
┌─────────────────────────────────────────────┐
│  SolidJS Frontend                           │
│  Device panels · Spectrum · Logs            │
└──────────┬──────────────────────┬────────────┘
           │ invoke()            │ listen()
           ▼                     │
┌──────────────────────────────────────────────┐
│  Tauri Rust Backend                          │
│  voice.rs (cpal + FFT) · control.rs · state  │
│  ┌─────────┐  ┌──────────┐  ┌────────────┐  │
│  │实时轨    │  │指令轨     │  │Daemon管理  │  │
│  │60Hz FFT │  │VLM Client│  │健康检查    │  │
│  └─────────┘  └────┬─────┘  └─────┬──────┘  │
└────────────────────┼──────────────┼──────────┘
                     │ HTTP         │ spawn
                     ▼              ▼
              ┌──────────────────────────┐
              │  Python Daemon (Flask)   │
              │  /vlm → ai_engine       │
              │  Base64 audio → JSON    │
              └──────────────────────────┘
```

The Rust backend spawns a Python Flask daemon as a child process at startup. Voice audio is captured via `cpal`, processed through a dual-track pipeline (real-time FFT for visualization + deferred recording for VLM), and the daemon returns structured commands that update device state.

## Prerequisites

- [Bun](https://bun.sh) (or npm/pnpm)
- [Rust](https://rustup.rs) toolchain
- [uv](https://docs.astral.sh/uv/) — Python package manager (handles venv creation automatically)
- [Python = 3.14](https://python.org)

> [!IMPORTANT]
> The Rust build **must** go through `uv` to link against the correct Python in `python_engine/.venv`. Running `cargo build` directly will fail with PyO3 linker errors.

## Getting Started

Clone and install dependencies:

```bash
git clone https://github.com/your-username/visualised-intelligent-voice-interaction-control-system.git
cd visualised-intelligent-voice-interaction-control-system
bun install
```

The Python environment is created automatically on first run. Start the full application:

```bash
bun run dev
```

This launches the Vite dev server, Tauri backend, and Python daemon together.

## Development

### Build Commands

```bash
# Full application (frontend + Tauri + Python daemon)
bun run dev                    # Development mode
bun run build                  # Production build

# Rust backend only (requires uv for PyO3)
cd src-tauri
uv run --project ../python_engine cargo build
uv run --project ../python_engine cargo check
uv run --project ../python_engine cargo test --lib
```

### Running a Single Test

```bash
cd src-tauri && uv run --project ../python_engine cargo test --lib -- test_parse_turn_on_light
```

## Project Structure

```
├── src/                        # SolidJS frontend
│   ├── components/
│   │   ├── devices/            # Light, AirCondition, Fan, Curtain panels
│   │   ├── rooms/              # Room layout components
│   │   └── ui/                 # LiquidGlass, MicButton, toggles
│   ├── store/deviceStore.ts    # Reactive device state (SolidJS signals)
│   └── App.tsx                 # Root layout
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── lib.rs              # Tauri entry, commands, Python env init
│   │   ├── voice.rs            # cpal audio capture + dual-track pipeline
│   │   ├── control.rs          # Command parsing (exact → regex → fuzzy)
│   │   ├── state.rs            # Device state types (Arc<RwLock<>>)
│   │   ├── daemon_manager.rs   # Python process lifecycle
│   │   └── logger.rs           # Log forwarding via Tauri events
│   └── build.rs                # PyO3 env detection at compile time
└── python_engine/              # Python VLM service
    ├── daemon.py               # Flask HTTP server (port 8765)
    ├── ai_engine.py            # VLM inference logic
    └── pyproject.toml          # Python dependencies (uv managed)
```

## How It Works

### Voice Command Pipeline

1. **Capture** — `cpal` records audio from the default microphone at 48kHz
2. **Real-time Track** — Audio samples are FFT-processed and emitted as `spectrum-update` events at 60Hz for the visualization panel
3. **Recording Track** — When the user presses and holds the mic button, audio accumulates in a buffer
4. **VLM Processing** — On release, the buffered audio is Base64-encoded and POSTed to the Python daemon at `http://127.0.0.1:8765/vlm`
5. **Command Extraction** — The daemon calls the VLM, which returns a structured JSON command with device, action, and confidence score
6. **Execution** — The Rust backend parses the response, validates confidence (>0.6 threshold), and updates the shared `DeviceState`
7. **UI Update** — The new state is broadcast to the frontend via the `device-state-changed` Tauri event

### Supported Commands

| Device | Actions |
|--------|---------|
| Light (灯) | 开/关, brightness 0-100 |
| Air Conditioner (空调) | 开/关, temperature 16-30°C |
| Fan (风扇) | 开/关, speed 0-3 |
| Curtain (窗帘) | 开/关 |
| All (全部) | 开/关 all devices |

Commands can specify a room: "卧室开灯", "客厅空调调到25度". Without a room prefix, the living room is used by default.

## License

[MIT](LICENSE)
