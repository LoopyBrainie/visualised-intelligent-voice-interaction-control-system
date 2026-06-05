// voice.rs - 语音采集与频谱处理模块
// 架构: cpal 采集 → 双轨并行处理
//   实时轨 (60Hz): mpsc channel → FftProcessor → spectrum-update 事件
//   指令轨 (Recording Mode): RecordingBuffer → VlmClient → Python Daemon → parse_vlm_response → execute_command
//   录制模式: 点击开始 → 累积音频 → 点击停止 → 发送完整音频

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tokio::time::{timeout, Duration as TokioDuration};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, FftPlanner};
use tauri::{AppHandle, Emitter};
use crate::error::AppError;
use crate::logger::{log_error_ts, log_info_ts, log_warn_ts, new_trace_id};

use crate::control;

/// VLM 消息类型 - 支持音频数据和停止信号
enum VlmMessage {
    Audio(Vec<i16>, String),
    Stop,
}

/// 录制状态 - 控制是否累积音频用于发送
pub struct RecordingState {
    pub enabled: Arc<AtomicBool>,
    pub pending_audio: Arc<Mutex<Vec<i16>>>,
}

impl RecordingState {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            pending_audio: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 开始录制 - 清空缓冲区并启用录制
    pub fn start_recording(&self) {
        self.pending_audio.lock().map(|mut guard| guard.clear()).ok();
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// 停止录制 - 禁用录制并返回累积的音频
    pub fn stop_recording(&self) -> Vec<i16> {
        self.enabled.store(false, Ordering::SeqCst);
        self.pending_audio.lock().map(|mut guard| guard.drain(..).collect()).unwrap_or_default()
    }

    /// 检查是否正在录制
    pub fn is_recording(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
}

/// 麦克风设备信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct MicrophoneDevice {
    pub name: String,
    pub sample_rates: Vec<u32>,
}

/// 频谱处理器 (FFT + Overlap)
pub struct FftProcessor {
    fft_size: usize,
    buffer: Vec<i16>,
    overlap_samples: usize,
    planner: FftPlanner<f32>,
}

impl FftProcessor {
    pub fn new(fft_size: usize) -> Self {
        Self {
            fft_size,
            buffer: Vec::with_capacity(fft_size * 2),
            overlap_samples: fft_size / 2, // 50% overlap = 256 samples (for 512 FFT)
            planner: FftPlanner::new(),
        }
    }

    /// 处理新样本，返回频谱数据
    pub fn process(&mut self, new_samples: &[i16]) -> Vec<f32> {
        // 1. 将新样本追加到缓冲区
        self.buffer.extend_from_slice(new_samples);

        // 2. 保持固定大小的 Sliding Window：超过 fft_size 时丢弃旧样本
        if self.buffer.len() > self.fft_size {
            let drain_count = self.buffer.len() - self.fft_size;
            self.buffer.drain(0..drain_count);
        }

        // 3. 如果缓冲区满了，执行 FFT
        if self.buffer.len() >= self.fft_size {
            let spectrum = {
                let samples = self.buffer[..self.fft_size].to_vec();
                self.compute_spectrum(&samples)
            };
            spectrum
        } else {
            vec![0.0; self.fft_size / 2]
        }
    }

    fn compute_spectrum(&mut self, samples: &[i16]) -> Vec<f32> {
        let n = self.fft_size;
        let pi2 = 2.0 * std::f32::consts::PI;

        // 转换为复数并加汉宁窗
        let mut buf: Vec<Complex<f32>> = samples
            .iter()
            .map(|&s| Complex::new(s as f32 / i16::MAX as f32, 0.0))
            .collect();

        for (i, c) in buf.iter_mut().enumerate() {
            // 汉宁窗: 0.5 * (1 - cos(2*PI*i / (N-1)))
            let window = 0.5 * (1.0 - (pi2 * i as f32 / (n - 1) as f32).cos());
            *c *= window;
        }

        let fft = self.planner.plan_fft_forward(n);
        fft.process(&mut buf);

        // 取 Nyquist 频率以内的幅度，对数缩放 (dB)
        let spectrum: Vec<f32> = buf.iter()
            .take(n / 2)
            .map(|c| {
                let amplitude = (c.re * c.re + c.im * c.im).sqrt();
                let db = 20.0 * (amplitude.max(1e-6)).log10();
                ((db + 60.0) / 60.0).max(0.0).min(1.0)
            })
            .collect();
        spectrum
    }
}

/// FFT 频谱事件载荷 - 包含频谱数据和元信息
#[derive(serde::Serialize, Clone)]
struct SpectrumPayload {
    frequencies: Vec<f32>,
    sample_rate: u32,
    fft_size: usize,
}

// ============================================================
// VLM HTTP 客户端 (指令轨)
// ============================================================

// ============================================================
// VLM HTTP 客户端 (指令轨)
// ============================================================

/// VLM 客户端 - 通过 HTTP 与 Python Daemon 通信
pub struct VlmClient {
    client: reqwest::Client,
    daemon_url: String,
}

impl VlmClient {
    pub fn new(daemon_url: &str) -> Self {
        // 配置 client 以避免 Windows 上的连接复用问题
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(35))
            .pool_max_idle_per_host(1)  // 限制每个 host 的空闲连接数
            .http1_only()  // 强制使用 HTTP/1.1，避免连接复用复杂性
            .build()
            .unwrap_or_default();

        Self {
            client,
            daemon_url: daemon_url.to_string(),
        }
    }

    /// 发送音频到 VLM Daemon 并返回响应
    /// 超时时间: 35秒（内层网络超时，外层 tokio 兜底 37s）
    /// trace_id 用于全链路追踪
    /// sample_rate: 音频原生采样率（Hz），用于 WAV 编码
    pub async fn send_audio(&self, audio_samples: &[i16], prompt: &str, trace_id: &str, sample_rate: u32) -> Result<String, AppError> {
        use crate::logger::log_info_ts;

        // 1. i16 PCM → u8 bytes (Little Endian)
        let pcm_bytes: Vec<u8> = audio_samples
            .iter()
            .flat_map(|&s| s.to_le_bytes())
            .collect();

        // 2. Base64 编码
        let audio_b64 = BASE64.encode(&pcm_bytes);

        // 3. 构造请求体（包含 trace_id 和 sample_rate 用于追踪和 WAV 编码）
        let payload = serde_json::json!({
            "audio": audio_b64,
            "prompt": prompt,
            "trace_id": trace_id,
            "sample_rate": sample_rate
        });

        log_info_ts(&format!("[→] VLM 请求发出，音频大小: {} bytes ({} samples)", pcm_bytes.len(), audio_samples.len()), trace_id, "voice");

        // 4. HTTP POST with 35s timeout
        let request = self
            .client
            .post(format!("{}/vlm", self.daemon_url))
            .json(&payload)
            .send();

        match timeout(TokioDuration::from_secs(37), request).await {
            Ok(Ok(response)) => {
                log_info_ts("[←] VLM 响应收到", trace_id, "voice");
                response
                    .text()
                    .await
                    .map_err(|e| AppError::network_error(format!("读取响应失败: {}", e)))
            }
            Ok(Err(e)) => Err(AppError::network_error(format!("HTTP 请求失败: {}", e))),
            Err(_) => Err(AppError::vlm_timeout()),
        }
    }
}

/// VoiceManager - 管理和维护语音采集生命周期
pub struct VoiceManager {
    is_running: Arc<AtomicBool>,
    recording_state: Arc<RecordingState>,
    language_mode: Arc<AtomicBool>,  // 语言模式标志（语言模式时进行FFT，手势模式时暂停）
    stream_paused: Arc<AtomicBool>,  // 流暂停状态
    vlm_tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<VlmMessage>>>>,
    stream: Arc<Mutex<cpal::Stream>>,  // 改为可控制的流
}

impl VoiceManager {
    /// 启动语音采集
    /// - app: Tauri 应用句柄，用于发送事件
    /// - state: 全局状态，用于 VLM 指令执行
    /// 音频以设备原生格式采集，统一转换为 i16 后分发给 FFT 和 VLM 两条路径
    pub fn start(
        app: AppHandle,
        state: crate::state::GlobalState,
    ) -> Result<Self, AppError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AppError::audio_device("未找到输入设备"))?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        println!("[Voice] 使用设备: {}", device_name);

        let config = device
            .default_input_config()
            .map_err(|e| AppError::audio_device(e.to_string()))?;

        let native_rate = config.sample_rate();
        let native_format = config.sample_format();
        let native_channels = config.channels();
        println!(
            "[Voice] 原生配置: {} Hz, {:?}, {} ch",
            native_rate, native_format, native_channels
        );

        // 收集支持的输入配置（用于选择 48kHz 避免 WASAPI 重采样）
        let supported_configs: Vec<_> = device.supported_input_configs()
            .map(|c| c.collect())
            .unwrap_or_default();

        // 语言模式（默认开启，手势模式时暂停流）
        let language_mode = Arc::new(AtomicBool::new(true));
        let language_mode_clone = language_mode.clone();

        // 流暂停状态
        let stream_paused = Arc::new(AtomicBool::new(false));
        let stream_paused_clone = stream_paused.clone();

        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = is_running.clone();

        // 录制状态 - 控制是否累积音频
        let recording_state = Arc::new(RecordingState::new());

        // 创建有界通道 (cpal callback → 实时轨处理线程)
        // sync_channel(64) 约缓冲 1.3 秒音频，try_send 保证 cpal 回调永不阻塞
        let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<Vec<i16>>(64);

        // 构建流配置
        // 关键: WASAPI 共享模式下，音频引擎通常以 48kHz 运行
        // 若请求不同速率（如设备原生 16kHz），WASAPI 会静默重采样导致混叠失真
        // 解决: 选择设备支持范围内最接近 48kHz 的速率和格式，避免隐式重采样
        let desired_rate = 48000u32;
        // 优先: 同格式 + 48kHz
        // 次选: 任意格式 + 48kHz (会切换格式)
        // 回退: 设备原生配置
        let best_config = supported_configs.iter()
            .find(|c| c.sample_format() == native_format
                && desired_rate >= c.min_sample_rate()
                && desired_rate <= c.max_sample_rate())
            .or_else(|| supported_configs.iter()
                .find(|c| desired_rate >= c.min_sample_rate()
                    && desired_rate <= c.max_sample_rate()));
        let (stream_rate, stream_format) = if let Some(best) = best_config {
            (desired_rate, best.sample_format())
        } else {
            (native_rate, native_format)
        };
        let mut stream_config: cpal::StreamConfig = config.into();
        stream_config.sample_rate = stream_rate;
        stream_config.buffer_size = cpal::BufferSize::Default;
        println!("[Voice] 流配置: {} Hz, {:?}, {} ch, buffer_size={:?}",
            stream_rate, stream_format, stream_config.channels, stream_config.buffer_size);

        let channels = stream_config.channels as usize;
        let stream = match stream_format {
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if running_clone.load(Ordering::Relaxed) && !stream_paused_clone.load(Ordering::Relaxed) {
                            let mono = if channels > 1 {
                                data.chunks_exact(channels)
                                    .map(|frame| (frame.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16)
                                    .collect()
                            } else {
                                data.to_vec()
                            };
                            let _ = audio_tx.try_send(mono);
                        }
                    },
                    |err| eprintln!("[Voice] 音频流错误: {}", err),
                    None,
                )
            }
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if running_clone.load(Ordering::Relaxed) && !stream_paused_clone.load(Ordering::Relaxed) {
                            let mono_f32: Vec<f32> = if channels > 1 {
                                data.chunks_exact(channels)
                                    .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                                    .collect()
                            } else {
                                data.to_vec()
                            };
                            let i16_data: Vec<i16> = mono_f32.iter()
                                .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                                .collect();
                            let _ = audio_tx.try_send(i16_data);
                        }
                    },
                    |err| eprintln!("[Voice] 音频流错误: {}", err),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if running_clone.load(Ordering::Relaxed) && !stream_paused_clone.load(Ordering::Relaxed) {
                            let i16_data: Vec<i16> = if channels > 1 {
                                data.chunks_exact(channels)
                                    .map(|frame| {
                                        let avg = frame.iter().map(|&s| s as i32).sum::<i32>() / channels as i32;
                                        (avg - 32768) as i16
                                    })
                                    .collect()
                            } else {
                                data.iter().map(|&s| (s as i32 - 32768) as i16).collect()
                            };
                            let _ = audio_tx.try_send(i16_data);
                        }
                    },
                    |err| eprintln!("[Voice] 音频流错误: {}", err),
                    None,
                )
            }
            _ => return Err(AppError::audio_device(format!("不支持的音频格式: {:?}", native_format))),
        }
        .map_err(|e| AppError::audio_device(e.to_string()))?;

        let stream = Arc::new(Mutex::new(stream));
        let stream_for_init = stream.clone();

        stream.lock().map_err(AppError::from)?.play().map_err(|e| AppError::audio_device(e.to_string()))?;
        println!("[Voice] 音频流启动成功");

        // ================================================================
        // 双轨并行处理架构
        // ================================================================
        // 实时轨: cpal → rx → FFT → spectrum-update (60Hz)
        // 指令轨: RecordingState → vlm_tx → tokio task → VLM Daemon
        // 录制模式: 指令轨只在 recording_enabled=true 时累积，停止时发送完整音频
        // ================================================================

        let (vlm_tx, mut vlm_rx) = tokio::sync::mpsc::channel::<VlmMessage>(4);

        let app_for_vlm = app.clone();
        let app_for_fft = app.clone();
        let state_clone = state.clone();
        // 为 tokio task 和实时轨各创建一个克隆
        let recording_state_for_vlm = recording_state.clone();
        let recording_state_for_rt = recording_state.clone();
        let native_rate_for_vlm = stream_rate;  // 实际流采样率 (u32)，用于 WAV 编码
        let native_rate_for_fft = stream_rate;  // 实际流采样率 (u32)，用于 FFT 频率轴计算

        // ---------------------------------------------------------
        // 🚀 指令轨 (Command Track) - 依托 Tauri 内置 Tokio 运行时
        // ---------------------------------------------------------
        tauri::async_runtime::spawn(async move {
            let vlm_client = VlmClient::new("http://127.0.0.1:8765");

            while let Some(msg) = vlm_rx.recv().await {
                match msg {
                    VlmMessage::Audio(audio_data, trace_id) => {
                        // 通知前端 VLM 正在处理
                        let _ = app_for_vlm.emit("vlm-processing", true); // intentionally ignored: UI event, non-critical

                        log_info_ts("[处理开始] 收到音频数据，开始 VLM 识别", &trace_id, "voice");

                        match vlm_client.send_audio(&audio_data, "请提取设备控制指令", &trace_id, native_rate_for_vlm).await {
                            Ok(response) => {
                                log_info_ts(&format!("[VLM] 响应: {}", &response[..response.len().min(200)]), &trace_id, "voice");

                                // 解析 Python daemon 回传的日志并发送到前端
                                // logs 在 AppResult 格式中嵌套在 data.logs 下
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                                    if let Some(logs) = parsed.get("data").and_then(|d| d.get("logs")).and_then(|l| l.as_array()) {
                                        for log_entry in logs {
                                            if let Some(log_obj) = log_entry.as_object() {
                                                let _ts = log_obj.get("timestamp").and_then(|v| v.as_str()).unwrap_or("00:00:00.000");
                                                let level = log_obj.get("level").and_then(|v| v.as_str()).unwrap_or("INFO");
                                                let msg = log_obj.get("message").and_then(|v| v.as_str()).unwrap_or("");
                                                let src = log_obj.get("source").and_then(|v| v.as_str());
                                                let tid = log_obj.get("trace_id").and_then(|v| v.as_str()).unwrap_or(&trace_id);
                                                let entry = crate::logger::LogEntry::new(level, msg)
                                                    .with_trace_id(tid)
                                                    .with_source(src.unwrap_or("python"));
                                                crate::logger::log_proxy(&entry);
                                            }
                                        }
                                    }
                                }

                                if let Ok(cmd) = control::parse_vlm_response(&response) {
                                    log_info_ts(&format!("[解析] device={:?} action={:?} room={:?}",
                                        cmd.device, cmd.action, cmd.room), &trace_id, "voice");
                                    if let Err(e) = control::execute_vlm_command(&cmd, &state_clone) {
                                        log_error_ts(&format!("[执行失败] {}", e), &trace_id, "voice");
                                    } else {
                                        log_info_ts("[执行成功] 设备状态已更新", &trace_id, "voice");
                                        if let Ok(guard) = state_clone.read() {
                                            let _ = app_for_vlm.emit("device-state-changed", (*guard).clone()); // intentionally ignored: UI event
                                        }
                                    }
                                } else {
                                    log_warn_ts(&format!("[解析失败] VLM 响应格式错误: {}", &response[..100]), &trace_id, "voice");
                                }
                            }
                            Err(e) => {
                                log_error_ts(&format!("[VLM错误] {}", e), &trace_id, "voice");
                            }
                        }

                        let _ = app_for_vlm.emit("vlm-processing", false); // intentionally ignored: UI event
                    }
                    VlmMessage::Stop => {
                        println!("[Voice] 收到 Stop 信号，录制状态={}", recording_state_for_vlm.is_recording());
                        // 收到停止信号，发送最终录制的音频
                        let audio_data = recording_state_for_vlm.stop_recording();
                        println!("[Voice] 获取音频，大小={} samples", audio_data.len());
                        if !audio_data.is_empty() {
                            let trace_id = new_trace_id();
                            log_info_ts(&format!("[停止] 发送最终音频，大小: {} samples", audio_data.len()), &trace_id, "voice");

                            // intentionally ignored: one-way UI notification, no recovery path
                            let _ = app_for_vlm.emit("vlm-processing", true);
                            log_info_ts("[VLM] 准备发送音频到 daemon...", &trace_id, "voice");
                            match vlm_client.send_audio(&audio_data, "请提取设备控制指令", &trace_id, native_rate_for_vlm).await {
                                Ok(response) => {
                                    log_info_ts(&format!("[VLM] 最终响应: {}", &response[..response.len().min(200)]), &trace_id, "voice");

                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                                        if let Some(logs) = parsed.get("data").and_then(|d| d.get("logs")).and_then(|l| l.as_array()) {
                                            for log_entry in logs {
                                                if let Some(log_obj) = log_entry.as_object() {
                                                    let level = log_obj.get("level").and_then(|v| v.as_str()).unwrap_or("INFO");
                                                    let msg = log_obj.get("message").and_then(|v| v.as_str()).unwrap_or("");
                                                    let src = log_obj.get("source").and_then(|v| v.as_str());
                                                    let tid = log_obj.get("trace_id").and_then(|v| v.as_str()).unwrap_or(&trace_id);
                                                    let entry = crate::logger::LogEntry::new(level, msg)
                                                        .with_trace_id(tid)
                                                        .with_source(src.unwrap_or("python"));
                                                    crate::logger::log_proxy(&entry);
                                                }
                                            }
                                        }
                                    }

                                    if let Ok(cmd) = control::parse_vlm_response(&response) {
                                        log_info_ts(&format!("[解析] device={:?} action={:?} room={:?}",
                                            cmd.device, cmd.action, cmd.room), &trace_id, "voice");
                                        if let Err(e) = control::execute_vlm_command(&cmd, &state_clone) {
                                            log_error_ts(&format!("[执行失败] {}", e), &trace_id, "voice");
                                        } else {
                                            log_info_ts("[执行成功] 设备状态已更新", &trace_id, "voice");
                                            if let Ok(guard) = state_clone.read() {
                                                // intentionally ignored: one-way UI notification, no recovery path
                                                let _ = app_for_vlm.emit("device-state-changed", (*guard).clone());
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    log_error_ts(&format!("[VLM错误] {}", e), &trace_id, "voice");
                                }
                            }
                            log_info_ts("[VLM] 处理完成，停止 processing 状态", &trace_id, "voice");
                            // intentionally ignored: one-way UI notification, no recovery path
                            let _ = app_for_vlm.emit("vlm-processing", false);
                        } else {
                            println!("[Voice] 音频为空，跳过 VLM 发送");
                        }
                        println!("[Voice] 指令轨收到停止信号，等待下一次录制");
                    }
                }
            }
            println!("[Voice] 指令轨线程结束");
        });

        // ---------------------------------------------------------
        // ⚡ 实时轨 (Real-time Track) - 独立线程，处理 FFT 与缓冲
        // ---------------------------------------------------------
        let running_clone = is_running.clone();
        std::thread::spawn(move || {
            let mut fft_processor = FftProcessor::new(512);

            while running_clone.load(Ordering::Relaxed) {
                match audio_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(samples) => {
                        // FFT 路径：原生采样率音频，无重采样
                        if language_mode_clone.load(Ordering::SeqCst) {
                            let spectrum = fft_processor.process(&samples);
                            let payload = SpectrumPayload {
                                frequencies: spectrum,
                                sample_rate: native_rate_for_fft,
                                fft_size: 512,
                            };
                            // intentionally ignored: one-way UI notification, no recovery path
                            let _ = app_for_fft.emit("spectrum-update", payload);
                        }
                        // 手势模式时：完全不执行 FFT，节省 CPU

                        // VLM 录制路径：无损累积原生采样率音频
                        if recording_state_for_rt.is_recording() {
                            if let Ok(mut guard) = recording_state_for_rt.pending_audio.try_lock() {
                                guard.extend_from_slice(&samples);
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // 超时继续等待，这是正常的
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }
            println!("[Voice] 实时轨线程结束");
        });

        Ok(Self {
            is_running,
            recording_state,
            language_mode,
            stream_paused,
            vlm_tx: Arc::new(Mutex::new(Some(vlm_tx))),
            stream: stream_for_init,
        })
    }

    /// 停止语音采集
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
        println!("[Voice] 语音采集已停止");
    }

    /// 开始录制 - 启用录制模式，清空缓冲区
    pub fn start_recording(&self) {
        self.recording_state.start_recording();
        println!("[Voice] 录制已开始");
    }

    /// 停止录制并发送完整音频
    pub fn stop_recording_and_send(&self) {
        // 只发送停止信号，不先获取音频
        if let Ok(guard) = self.vlm_tx.try_lock() {
            if let Some(ref tx) = *guard {
                match tx.try_send(VlmMessage::Stop) {
                    Ok(()) => println!("[Voice] 停止信号已发送到 VLM 任务"),
                    Err(e) => {
                        println!("[Voice] 停止信号发送失败: {:?}", e);
                        // 尝试在同步上下文直接处理（fallback）
                        drop(guard);
                        if let Ok(guard2) = self.vlm_tx.try_lock() {
                            if let Some(ref tx2) = *guard2 {
                                let _ = tx2.try_send(VlmMessage::Stop);
                            }
                        }
                    }
                };
            }
        }
    }

    /// 检查是否正在录制
    pub fn is_recording(&self) -> bool {
        self.recording_state.is_recording()
    }

    /// 设置语言模式
    /// enabled=true: 语言模式（开启 FFT 和音频采集）
    /// enabled=false: 手势模式（暂停 FFT 和音频流以节省功耗）
    pub fn set_language_mode(&self, enabled: bool) {
        self.language_mode.store(enabled, Ordering::SeqCst);
        println!("[Voice] 语言模式已设置为: {}", if enabled { "开启" } else { "关闭" });

        // 控制音频流暂停/恢复
        let should_pause = !enabled;
        let currently_paused = self.stream_paused.load(Ordering::SeqCst);

        if should_pause && !currently_paused {
            // 暂停流
            if let Ok(guard) = self.stream.lock() {
                guard.pause().ok();
                self.stream_paused.store(true, Ordering::SeqCst);
                println!("[Voice] 音频流已暂停");
            }
        } else if !should_pause && currently_paused {
            // 恢复流
            if let Ok(guard) = self.stream.lock() {
                guard.play().ok();
                self.stream_paused.store(false, Ordering::SeqCst);
                println!("[Voice] 音频流已恢复");
            }
        }
    }

    /// 获取当前语言模式状态
    pub fn is_language_mode(&self) -> bool {
        self.language_mode.load(Ordering::SeqCst)
    }
}

/// 列出所有可用麦克风设备
pub fn list_devices() -> Vec<MicrophoneDevice> {
    let host = cpal::default_host();
    host.devices()
        .unwrap()
        .filter_map(|device| {
            let name = device.name().ok()?;
            Some(MicrophoneDevice {
                name,
                sample_rates: vec![], // cpal 0.17 不提供 supported_sample_rates
            })
        })
        .collect()
}

/// 获取默认输入设备名称
pub fn get_default_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_input_device()
        .and_then(|d| d.name().ok())
}
