// gesture.rs - 手势识别与摄像头采集模块
// 架构:
//   Camera Thread (nokhwa) → FrameBuffer (latest JPEG only)
//   Gesture Worker Thread → Python Daemon /gesture → GestureFsm → control.rs
//   Frontend preview via get_camera_frame Tauri command + data URL (rAF polling)
//
// 设计决策（来自 grill-me 深度问答）:
//   - 停留确认 (Dwell): DWELL_TIMEOUT=1500ms 后触发动作
//   - 切换冷却: SWITCH_COOLDOWN=800ms (指点切换设备), ACTION_COOLDOWN=1500ms (执行动作)
//   - Jitter 容忍: JITTER_TOLERANCE=300ms 内丢帧不重置 dwell 进度
//   - 锁粒度: JPEG 编码在锁外，仅在替换 Arc 指针瞬间加锁
//   - 前端计时: 绝对时间戳模式，Rust 发送 deadline，前端倒计时

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType, FrameFormat};
use nokhwa::pixel_format::RgbFormat;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::control::{self, CommandType, TargetDevice, TargetRoom, ParsedCommand};
use crate::error::AppError;
use crate::logger::{log_error, log_info, log_warn};
use crate::state::GlobalState;

// ============================================================
// 常量 (grill-me 阶段确认)
// ============================================================

/// 停留确认时间: 同一手势持续 1500ms 后触发动作
const DWELL_TIMEOUT_MS: u64 = 1500;
/// 设备切换冷却: 一指手势切换选中设备的最小间隔
const SWITCH_COOLDOWN_MS: u64 = 800;
/// 动作冷却: 执行设备动作后的最小间隔
const ACTION_COOLDOWN_MS: u64 = 1500;
/// Jitter 容忍: MediaPipe 偶发丢帧不重置 dwell 进度
const JITTER_TOLERANCE_MS: u64 = 300;
/// 手势识别轮询间隔 (10Hz = 100ms)
const GESTURE_POLL_INTERVAL_MS: u64 = 100;
/// 摄像头健康探测间隔
const CAMERA_PROBE_INTERVAL_MS: u64 = 30_000;
/// 摄像头目标分辨率
const CAMERA_WIDTH: u32 = 640;
const CAMERA_HEIGHT: u32 = 480;
/// 摄像头目标帧率
const CAMERA_FPS: u32 = 30;

/// Python Daemon 地址
const DAEMON_URL: &str = "http://127.0.0.1:8765";

/// 设备切换顺序（伸出一根手指手势轮转, 标签 "one"）
/// 关键：粒度为"(房间, 设备类型)"二元组,不是设备类型本身
/// 原因:state 里有 8 个具体设备实例(2 房间 × 4 类型),如果只轮转
/// 设备类型,"灯"这个标签对应 2 个实例(客厅灯 + 卧室灯),
/// 一指 toggle 时无法表达"我现在指向的是哪个具体灯"。
/// 伸出两根手指(标签 "two")只切当前选中的那一个,绝不连带同类型其它实例。
/// 三指/四指的全开/全关快捷手势不受此粒度影响(走独立 action 路径)。
/// 五指(标签 "five")走 ToggleAll,与 DEVICE_CYCLE 无关。
const DEVICE_CYCLE: &[(TargetRoom, TargetDevice)] = &[
    (TargetRoom::LivingRoom, TargetDevice::Light),
    (TargetRoom::LivingRoom, TargetDevice::AirCondition),
    (TargetRoom::LivingRoom, TargetDevice::Fan),
    (TargetRoom::LivingRoom, TargetDevice::Curtain),
    (TargetRoom::Bedroom, TargetDevice::Light),
    (TargetRoom::Bedroom, TargetDevice::AirCondition),
    (TargetRoom::Bedroom, TargetDevice::Fan),
    (TargetRoom::Bedroom, TargetDevice::Curtain),
];

// ============================================================
// FrameBuffer — 存储最新一帧 JPEG，始终只保留最新帧（背压保护）
// ============================================================

/// "最新帧"采样策略: 始终只存最新一帧 JPEG，旧帧直接丢弃
/// JPEG 编码在锁外完成（耗时操作），仅在替换指针瞬间加锁
#[derive(Clone)]
pub struct FrameBuffer {
    inner: Arc<Mutex<Option<Vec<u8>>>>,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(None)) }
    }

    /// 存储新帧（锁内仅替换指针）
    pub fn store(&self, jpeg: Vec<u8>) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(jpeg);
        }
    }

    /// 读取最新帧（克隆，锁内仅读取 + clone）
    pub fn latest(&self) -> Option<Vec<u8>> {
        self.inner.lock().ok().and_then(|g| g.clone())
    }
}

// ============================================================
// GestureClient — HTTP 与 Python Daemon /gesture 通信
// ============================================================

/// 通过 HTTP POST 发送 JPEG 帧到 Python Daemon 进行手势识别
pub struct GestureClient {
    client: reqwest::blocking::Client,
    daemon_url: String,
    consecutive_failures: Arc<Mutex<u32>>,
}

impl GestureClient {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .pool_max_idle_per_host(1)
            .build()
            .unwrap_or_default();

        Self {
            client,
            daemon_url: DAEMON_URL.to_string(),
            consecutive_failures: Arc::new(Mutex::new(0)),
        }
    }

    /// 发送 JPEG 帧到 Python Daemon，返回手势识别结果
    pub fn recognize(&self, jpeg_bytes: &[u8]) -> Result<GestureResult, AppError> {
        match self
            .client
            .post(format!("{}/gesture", self.daemon_url))
            .header("Content-Type", "image/jpeg")
            .body(jpeg_bytes.to_vec())
            .send()
        {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    // 先读文本再反序列化，以便在解析失败时记录响应体
                    match response.text() {
                        Ok(body) => {
                            match serde_json::from_str::<GestureResult>(&body) {
                                Ok(result) => {
                                    self.reset_failures();
                                    Ok(result)
                                }
                                Err(e) => {
                                    self.record_failure();
                                    let preview: String = body.chars().take(300).collect();
                                    log_warn(&format!(
                                        "[Gesture] JSON 解析失败: {} | body: {}",
                                        e, preview
                                    ));
                                    Err(AppError::gesture_error(format!(
                                        "解析手势响应失败: {} | body: {}",
                                        e, preview
                                    )))
                                }
                            }
                        }
                        Err(e) => {
                            self.record_failure();
                            log_warn(&format!("[Gesture] 读取响应体失败: {}", e));
                            Err(AppError::network_error(format!("读取手势响应失败: {}", e)))
                        }
                    }
                } else {
                    let body = response.text().unwrap_or_default();
                    self.record_failure();
                    log_warn(&format!("[Gesture] HTTP {}: {}", status, body));
                    Err(AppError::gesture_error(format!("手势服务 HTTP {}: {}", status, body)))
                }
            }
            Err(e) => {
                self.record_failure();
                log_warn(&format!("[Gesture] HTTP 请求失败: {}", e));
                Err(AppError::network_error(format!("手势服务不可达: {}", e)))
            }
        }
    }

    fn record_failure(&self) {
        if let Ok(mut guard) = self.consecutive_failures.lock() {
            *guard += 1;
        }
    }

    fn reset_failures(&self) {
        if let Ok(mut guard) = self.consecutive_failures.lock() {
            *guard = 0;
        }
    }

    /// 连续失败次数（用于触发 Daemon 自动重启）
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.lock().map(|g| *g).unwrap_or(0)
    }
}

/// MediaPipe 手部关键点 (归一化坐标 0.0~1.0)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Landmark {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Python Daemon /gesture 返回的 JSON 结构
#[derive(Debug, Clone, Deserialize)]
pub struct GestureResult {
    /// 伸出的手指数量, 0..5
    /// None 表示未检测到手
    pub count: Option<u8>,
    pub confidence: f64,
    #[serde(default)]
    pub landmarks: Vec<Landmark>,
}

// ============================================================
// CameraInfo — 摄像头列表（前端选择用）
// ============================================================

/// 可用摄像头信息（序列化到前端用于下拉选择）
#[derive(Debug, Clone, Serialize)]
pub struct CameraInfo {
    /// 摄像头唯一标识符（human_name，用于 CameraIndex::String）
    pub id: String,
    /// 人类可读名称
    pub name: String,
    /// 后端描述（如 "USB Camera"）
    pub description: String,
}

/// 枚举系统可用摄像头，返回 CameraInfo 列表
/// id 使用数字索引（与 nokhwa 枚举顺序一致），而非 human_name
/// 因为 Windows 上 CameraIndex::String 不接受人类可读名称
pub fn list_cameras() -> Vec<CameraInfo> {
    nokhwa::query(nokhwa::utils::ApiBackend::Auto)
        .map(|cameras| {
            cameras
                .into_iter()
                .enumerate()
                .map(|(i, c)| CameraInfo {
                    id: i.to_string(),
                    name: c.human_name().to_string(),
                    description: c.description().to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

// ============================================================
// GestureHealth + InteractionContext
// ============================================================

#[derive(Debug, Clone, Serialize)]
pub struct GestureHealth {
    pub camera_ok: bool,
    pub python_ok: bool,
    pub camera_name: Option<String>,
}

impl Default for GestureHealth {
    fn default() -> Self {
        Self {
            camera_ok: false,
            python_ok: false,
            camera_name: None,
        }
    }
}

/// 交互上下文 — 正交于 DeviceState，跟踪手势交互状态
#[derive(Debug, Clone, Serialize)]
pub struct InteractionContext {
    /// 当前选中的设备类型标签
    pub selected_device: String,
    /// 当前检测到的手势: 0..5 = 伸出的手指数量, None = 未检测到手
    pub current_count: Option<u8>,
    /// Dwell 进度 0.0~1.0（前端倒计时用）
    pub dwell_progress: f32,
    /// Dwell 目标截止时间戳 (ms)，None 表示未在 dwell 中
    pub dwell_deadline_ms: Option<u64>,
    /// 是否检测到手
    pub is_tracking: bool,
    /// 上次确认的动作描述
    pub last_action: Option<String>,
    /// 健康状态
    pub health: GestureHealth,
    /// 手部关键点 (归一化坐标)
    pub landmarks: Vec<Landmark>,
}

impl Default for InteractionContext {
    fn default() -> Self {
        Self {
            selected_device: "light".to_string(),
            current_count: None,
            dwell_progress: 0.0,
            dwell_deadline_ms: None,
            is_tracking: false,
            last_action: None,
            health: GestureHealth::default(),
            landmarks: Vec::new(),
        }
    }
}

// ============================================================
// GestureFsm — 停留确认状态机
// ============================================================

#[derive(Debug)]
enum FsmState {
    /// 空闲：无手势或手势不稳定
    Idle,
    /// 停留中：同一手势持续检测，累积 dwell 进度
    Dwell {
        count: u8,
        started: Instant,
        deadline: Instant,
    },
    /// 已确认：动作已触发，进入冷却
    Cooldown {
        until: Instant,
    },
}

/// FSM 内部决策 — 将状态读取与修改分离，避免借用冲突
enum DwellDecision {
    Start(u8),
    Continue,
    Confirm(u8),
    Restart(u8),
}

pub struct GestureFsm {
    state: FsmState,
    selected_device_index: usize,
}

impl GestureFsm {
    pub fn new() -> Self {
        Self {
            state: FsmState::Idle,
            selected_device_index: 0, // 从 Light 开始
        }
    }

    /// 处理检测到的手势(手指数量 0..5)，返回需要执行的动作
    pub fn update(&mut self, count: Option<u8>) -> Option<GestureAction> {
        let now = Instant::now();

        match &self.state {
            FsmState::Cooldown { until } => {
                if now < *until {
                    return None; // 冷却中，忽略
                }
                self.state = FsmState::Idle;
                // 冷却结束，继续处理当前手势
                return self.update(count);
            }
            _ => {}
        }

        match count {
            None => {
                // 无手势 → Idle（除非在 Dwell 中且 jitter 容忍内）
                if let FsmState::Dwell { started, .. } = &self.state {
                    let elapsed = now.duration_since(*started);
                    // Jitter 容忍：短暂丢帧不重置
                    if elapsed < Duration::from_millis(JITTER_TOLERANCE_MS) {
                        return None;
                    }
                }
                self.state = FsmState::Idle;
                None
            }
            Some(count) => {
                self.handle_gesture(count, now)
            }
        }
    }

    fn handle_gesture(&mut self, count: u8, now: Instant) -> Option<GestureAction> {
        // 决策模式：先读取状态做决策（不可变借用），再执行（可变借用）
        let decision = match &self.state {
            FsmState::Idle => {
                DwellDecision::Start(count)
            }
            FsmState::Dwell { count: current, deadline, .. } => {
                if current == &count {
                    if now >= *deadline {
                        DwellDecision::Confirm(*current)
                    } else {
                        DwellDecision::Continue
                    }
                } else {
                    DwellDecision::Restart(count)
                }
            }
            FsmState::Cooldown { .. } => unreachable!(),
        };

        match decision {
            DwellDecision::Start(count) => {
                self.state = FsmState::Dwell {
                    count,
                    started: now,
                    deadline: now + Duration::from_millis(DWELL_TIMEOUT_MS),
                };
                None
            }
            DwellDecision::Continue => None,
            DwellDecision::Confirm(count) => {
                let action = self.resolve_action(count);
                let cooldown_ms = match action.as_ref() {
                    Some(GestureAction::SwitchDevice) => SWITCH_COOLDOWN_MS,
                    _ => ACTION_COOLDOWN_MS,
                };
                self.state = FsmState::Cooldown {
                    until: now + Duration::from_millis(cooldown_ms),
                };
                action
            }
            DwellDecision::Restart(count) => {
                self.state = FsmState::Dwell {
                    count,
                    started: now,
                    deadline: now + Duration::from_millis(DWELL_TIMEOUT_MS),
                };
                None
            }
        }
    }

    /// 将手势(手指数量)映射为动作
    /// count ∈ 0..5 与伸出的手指数量一一对应
    /// 1 指(拇指或食指均被 count_fingers 计为 1)→ 切换到下一个设备
    fn resolve_action(&mut self, count: u8) -> Option<GestureAction> {
        match count {
            0 => {
                // 握拳(零指) → 取消
                Some(GestureAction::Cancel)
            }
            1 => {
                // 一指 → 切换到下一个设备
                self.selected_device_index =
                    (self.selected_device_index + 1) % DEVICE_CYCLE.len();
                Some(GestureAction::SwitchDevice)
            }
            2 => {
                // 两指 → 开关当前选中的"具体设备实例"(不是设备类型)
                let (room, device) = self.selected_device();
                Some(GestureAction::ToggleDevice { room, device })
            }
            3 => {
                // 三指 → 打开所有设备
                Some(GestureAction::AllOn)
            }
            4 => {
                // 四指 → 关闭所有设备
                Some(GestureAction::AllOff)
            }
            5 => {
                // 五指(张开手掌) → 全开/全关切换 (取决于当前整体状态)
                Some(GestureAction::ToggleAll)
            }
            _ => None,
        }
    }

    /// 当前选中的具体设备（房间 + 类型）
    pub fn selected_device(&self) -> (TargetRoom, TargetDevice) {
        DEVICE_CYCLE[self.selected_device_index]
    }

    /// 获取当前 dwell 进度信息（前端展示用）
    pub fn dwell_info(&self) -> (Option<u8>, f32, Option<u64>) {
        match &self.state {
            FsmState::Dwell { count, started, deadline: _ } => {
                let total = DWELL_TIMEOUT_MS as f32;
                let elapsed = started.elapsed().as_millis() as f32;
                let progress = (elapsed / total).min(1.0);
                // deadline as absolute unix timestamp ms
                let abs_deadline = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
                    + (DWELL_TIMEOUT_MS - elapsed as u64);
                (Some(*count), progress, Some(abs_deadline))
            }
            _ => (None, 0.0, None),
        }
    }
}

/// 手势触发的动作
#[derive(Debug, Clone)]
pub enum GestureAction {
    /// 切换到下一个设备（一指）
    SwitchDevice,
    /// 切换指定 (room, device) 实例的开关状态（两指）
    /// 关键:必须携带房间维度,否则 toggle 客厅灯时会同时影响卧室灯
    ToggleDevice { room: TargetRoom, device: TargetDevice },
    /// 打开所有设备（三指）
    AllOn,
    /// 关闭所有设备（四指）
    AllOff,
    /// 切换所有设备的总开关状态（五指）:
    /// 若全部为关则全部打开,否则只要有一个开着就全部关闭
    ToggleAll,
    /// 取消（握拳/零指）
    Cancel,
}

impl std::fmt::Display for GestureAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GestureAction::SwitchDevice => write!(f, "切换设备"),
            GestureAction::ToggleDevice { room, device } => {
                write!(f, "开关 {:?}/{:?}", room, device)
            }
            GestureAction::AllOn => write!(f, "全部打开"),
            GestureAction::AllOff => write!(f, "全部关闭"),
            GestureAction::ToggleAll => write!(f, "全部切换"),
            GestureAction::Cancel => write!(f, "取消"),
        }
    }
}

// ============================================================
// CameraCapture — nokhwa 摄像头采集
// ============================================================

/// 选择最合适的摄像头格式：优先 MJPEG (原生 JPEG)，其次 YUYV
fn select_camera_format(camera: &mut nokhwa::Camera) -> Result<(), AppError> {
    let formats = camera.compatible_camera_formats()
        .map_err(|e| AppError::camera_error(format!("无法获取摄像头格式: {}", e)))?;

    if formats.is_empty() {
        return Err(AppError::camera_error("摄像头不支持任何格式"));
    }

    // 优先级: MJPEG 640x480@30 > YUYV 640x480@30 > 任意格式
    let mut best: Option<nokhwa::utils::CameraFormat> = None;
    let mut best_score = 0u32;

    for fmt in &formats {
        let (w, h) = (fmt.width(), fmt.height());
        let fps = fmt.frame_rate();
        let is_mjpeg = fmt.format() == FrameFormat::MJPEG;

        // 评分: MJPEG +2, 分辨率匹配 +3, 帧率匹配 +1
        let mut score = 0u32;
        if is_mjpeg { score += 2; }
        if w == CAMERA_WIDTH && h == CAMERA_HEIGHT { score += 3; }
        else if w <= 1280 && h <= 720 { score += 1; } // 合理分辨率
        if fps >= 15 && fps <= 30 { score += 1; }

        if score > best_score {
            best_score = score;
            best = Some(fmt.clone());
        }
    }

    if let Some(best_fmt) = best {
        log_info(&format!(
            "[Gesture] 选择摄像头格式: {:?} {}x{} @{}fps",
            best_fmt.format(), best_fmt.width(), best_fmt.height(), best_fmt.frame_rate()
        ));
        camera.set_camera_format(best_fmt)
            .map_err(|e| AppError::camera_error(format!("设置摄像头格式失败: {}", e)))?;
    }

    Ok(())
}

/// 将 nokhwa Buffer 转换为 JPEG bytes
/// `cam_width` / `cam_height`: 摄像头当前分辨率（nokhwa 0.10 Buffer 无 width/height 方法）
fn frame_to_jpeg(frame: &nokhwa::Buffer, cam_width: u32, cam_height: u32) -> Option<Vec<u8>> {
    match frame.source_frame_format() {
        FrameFormat::MJPEG => {
            // MJPEG 格式：buffer 本身已经是 JPEG
            Some(frame.buffer().to_vec())
        }
        _ => {
            // 非 JPEG 格式：用 image crate 编码
            let raw = frame.buffer();
            let rgb: Vec<u8> = raw.chunks(4)
                .flat_map(|chunk| {
                    if chunk.len() >= 4 {
                        vec![chunk[2], chunk[1], chunk[0]] // BGRA → RGB
                    } else if chunk.len() >= 3 {
                        vec![chunk[2], chunk[1], chunk[0]]
                    } else {
                        vec![0, 0, 0]
                    }
                })
                .collect();

            let mut jpeg_bytes = Vec::new();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 80);
            use image::ExtendedColorType;
            encoder.encode(&rgb, cam_width, cam_height, ExtendedColorType::Rgb8).ok()?;
            Some(jpeg_bytes)
        }
    }
}

// ============================================================
// GestureManager — 摄像头 + 手势识别生命周期管理
// ============================================================

/// 手势管理器：封装摄像头采集、手势识别、FSM、健康检查
pub struct GestureManager {
    is_running: Arc<AtomicBool>,
    frame_buffer: FrameBuffer,
    fsm: Arc<Mutex<GestureFsm>>,
    context: Arc<Mutex<InteractionContext>>,
    gesture_thread: Option<thread::JoinHandle<()>>,
    camera_thread: Option<thread::JoinHandle<()>>,
    camera_name: Option<String>,
}

impl GestureManager {
    /// 启动手势识别系统
    /// `camera_id`: 可选摄像头标识符（对应 CameraInfo.id），None 则使用第一个可用摄像头
    pub fn start(app: AppHandle, state: GlobalState, camera_id: Option<String>) -> Result<Self, AppError> {
        let is_running = Arc::new(AtomicBool::new(true));
        let frame_buffer = FrameBuffer::new();
        let fsm = Arc::new(Mutex::new(GestureFsm::new()));
        let context = Arc::new(Mutex::new(InteractionContext::default()));

        // start_camera 现在返回 (handle, camera_name, init_error)
        // init_error: 当摄像头初始化失败时携带 nokhwa 真实错误消息,
        // 之前这里被静默丢弃 → 前端 typedInvoke 总是收到 Ok → 错误横幅无法显示
        let (camera_thread, camera_name, camera_init_error) = Self::start_camera(
            frame_buffer.clone(),
            is_running.clone(),
            context.clone(),
            app.clone(),
            camera_id,
        );

        // 摄像头初始化失败时,清理已启动的线程并返回错误给前端
        if let Some(err_msg) = camera_init_error {
            is_running.store(false, Ordering::Relaxed);
            if let Some(h) = camera_thread {
                let _ = h.join();
            }
            return Err(AppError::camera_error(err_msg));
        }

        let gesture_thread = Self::start_gesture_worker(
            frame_buffer.clone(),
            fsm.clone(),
            context.clone(),
            is_running.clone(),
            app.clone(),
            state.clone(),
        );

        // 更新健康状态
        // camera_name.is_some() 反映摄像头实际初始化结果，
        // 而 camera_thread.is_some() 始终为 true（线程句柄永远存在）
        {
            let mut ctx = context.lock()
                .map_err(|e| AppError::from(e))?;
            ctx.health.camera_ok = camera_name.is_some();
            ctx.health.camera_name = camera_name.clone();
            ctx.health.python_ok = true;
        }

        // 仅在摄像头初始化成功时发射初始交互状态；
        // 失败时摄像头线程已发射 gesture-health-changed { camera_ok: false }，
        // 此处再发射会覆盖正确的健康状态。
        if camera_name.is_some() {
            if let Ok(ctx) = context.lock() {
                // intentionally ignored: one-way UI notification, no recovery path
                let _ = app.emit("interaction-state-changed", ctx.clone());
            }
        }

        log_info("[Gesture] 手势识别系统启动完成");

        Ok(Self {
            is_running,
            frame_buffer,
            fsm,
            context,
            gesture_thread,
            camera_thread,
            camera_name,
        })
    }

    /// 启动摄像头采集线程
    /// `camera_id`: 可选摄像头标识符，None 则使用 CameraIndex::Index(0)
    /// nokhwa Camera 不是 Send 的，必须在目标线程内创建
    ///
    /// 返回 (JoinHandle, camera_name, init_error):
    /// - camera_name: 摄像头初始化成功时为 Some(设备名)
    /// - init_error: 初始化失败/超时时为 Some(错误消息),GestureManager::start 据此返回 Err
    fn start_camera(
        frame_buffer: FrameBuffer,
        is_running: Arc<AtomicBool>,
        context: Arc<Mutex<InteractionContext>>,
        app: AppHandle,
        camera_id: Option<String>,
    ) -> (Option<thread::JoinHandle<()>>, Option<String>, Option<String>) {
        let (init_tx, init_rx) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || {
            // 在线程内初始化摄像头（nokhwa Camera 不是 Send，必须原地创建）
            // 使用 CameraIndex::Index(n) 而非 String，因为 Windows 上 String 不接受 human_name
            let camera_init = || -> Result<(nokhwa::Camera, String, (u32, u32)), AppError> {
                let index = match &camera_id {
                    Some(id) => CameraIndex::Index(id.parse::<u32>().unwrap_or(0)),
                    None => CameraIndex::Index(0),
                };
                // 使用 None 让摄像头返回默认格式 — 兼容性最好。
                // 之前用 AbsoluteHighestFrameRate + RgbFormat 的组合在 Windows 上
                // 经常失败:USB 摄像头通常只声明 MJPEG/YUYV,与 RgbFormat 求交集为空
                let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::None);
                let mut camera = nokhwa::Camera::new(index, requested)
                    .map_err(|e| AppError::camera_error(format!("无法打开摄像头: {}", e)))?;

                let name = camera.info().human_name().to_string();

                select_camera_format(&mut camera)?;
                camera.open_stream()
                    .map_err(|e| AppError::camera_error(format!("无法打开摄像头流: {}", e)))?;

                let fmt = camera.camera_format();
                let cam_resolution = (fmt.width() as u32, fmt.height() as u32);

                Ok((camera, name, cam_resolution))
            };

            match camera_init() {
                Ok((mut camera, name, cam_resolution)) => {
                    log_info(&format!("[Gesture] 摄像头已打开: {}", name));
                    // 更新健康状态（在 move 前 clone）
                    if let Ok(mut ctx) = context.lock() {
                        ctx.health.camera_ok = true;
                        ctx.health.camera_name = Some(name.clone());
                    }
                    // 通知主线程初始化成功
                    let _ = init_tx.send(Ok(name));

                    let frame_interval = Duration::from_millis(1000 / CAMERA_FPS as u64);

                    while is_running.load(Ordering::Relaxed) {
                        let frame_start = Instant::now();

                        match camera.frame() {
                            Ok(frame) => {
                                if let Some(jpeg) = frame_to_jpeg(&frame, cam_resolution.0, cam_resolution.1) {
                                    frame_buffer.store(jpeg);
                                }
                            }
                            Err(e) => {
                                log_warn(&format!("[Gesture] 摄像头采集错误: {}", e));
                                thread::sleep(Duration::from_millis(500));
                            }
                        }

                        let elapsed = frame_start.elapsed();
                        if elapsed < frame_interval {
                            thread::sleep(frame_interval - elapsed);
                        }
                    }

                    let _ = camera.stop_stream();
                    log_info("[Gesture] 摄像头线程退出");
                }
                Err(e) => {
                    log_error(&format!("[Gesture] 摄像头初始化失败: {}", e));
                    let _ = init_tx.send(Err(format!("{}", e)));

                    if let Ok(mut ctx) = context.lock() {
                        ctx.health.camera_ok = false;
                    }
                    // intentionally ignored: one-way UI notification, no recovery path
                    // python_ok=true: 摄像头初始化失败与 Python daemon 无关
                    let _ = app.emit("gesture-health-changed", GestureHealth {
                        camera_ok: false,
                        python_ok: true,
                        camera_name: None,
                    });
                }
            }
        });

        // 等待线程内摄像头初始化结果（最多 5 秒）
        // 关键: 把 nokhwa 真实错误消息向上传递,让 start_gesture_capture 返回 Err,
        // 前端 typedInvoke 才能在 toast 横幅中显示具体原因
        let (camera_name, init_error) = match init_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(name)) => (Some(name), None),
            Ok(Err(e)) => {
                log_error(&format!("[Gesture] 摄像头初始化报告失败: {}", e));
                (None, Some(e))
            }
            Err(_) => {
                let msg = "摄像头初始化超时（5s）".to_string();
                log_warn(&format!("[Gesture] {}", msg));
                (None, Some(msg))
            }
        };

        (Some(handle), camera_name, init_error)
    }

    /// 启动手势识别工作线程
    fn start_gesture_worker(
        frame_buffer: FrameBuffer,
        fsm: Arc<Mutex<GestureFsm>>,
        context: Arc<Mutex<InteractionContext>>,
        is_running: Arc<AtomicBool>,
        app: AppHandle,
        state: GlobalState,
    ) -> Option<thread::JoinHandle<()>> {
        let client = GestureClient::new();
        let mut last_probe = Instant::now();

        let handle = thread::spawn(move || {
            let mut last_jpeg_len: Option<usize> = None;

            while is_running.load(Ordering::Relaxed) {
                let loop_start = Instant::now();

                // 读取最新帧
                if let Some(jpeg_bytes) = frame_buffer.latest() {
                    let is_new = last_jpeg_len != Some(jpeg_bytes.len());
                    last_jpeg_len = Some(jpeg_bytes.len());

                    if is_new {
                        // 发送帧到 Python Daemon
                        match client.recognize(&jpeg_bytes) {
                            Ok(result) => {
                            // 更新 FSM
                            let action = {
                                let mut fsm_guard = match fsm.lock() {
                                    Ok(g) => g,
                                    Err(_) => {
                                        log_error("[Gesture] FSM 锁中毒 (action)");
                                        continue;
                                    }
                                };
                                fsm_guard.update(result.count)
                            };

                            // 更新交互上下文
                            {
                                let mut ctx = match context.lock() {
                                    Ok(g) => g,
                                    Err(_) => {
                                        log_error("[Gesture] 上下文锁中毒 (ctx)");
                                        continue;
                                    }
                                };
                                ctx.current_count = result.count;
                                ctx.is_tracking = result.count.is_some();
                                ctx.landmarks = result.landmarks.clone();
                                ctx.health.python_ok = true;

                                let (_dwell_gesture, dwell_progress, deadline_ms) = {
                                    let fsm_guard = match fsm.lock() {
                                        Ok(g) => g,
                                        Err(_) => {
                                            log_error("[Gesture] FSM 锁中毒 (dwell)");
                                            continue;
                                        }
                                    };
                                    fsm_guard.dwell_info()
                                };
                                ctx.dwell_progress = dwell_progress;
                                ctx.dwell_deadline_ms = deadline_ms;
                                ctx.selected_device = {
                                    let fsm_guard = match fsm.lock() {
                                        Ok(g) => g,
                                        Err(_) => {
                                            log_error("[Gesture] FSM 锁中毒 (device)");
                                            continue;
                                        }
                                    };
                                    let (room, device) = fsm_guard.selected_device();
                                    // 拼接为 "living_room.light" 这种带房间的 ID,
                                    // 表达"具体设备实例"而不是设备类型
                                    // 关键:必须用显式映射,不能用 `{:?}` + to_lowercase
                                    // —— TargetRoom::LivingRoom 的 Debug 输出是 "LivingRoom",
                                    // to_lowercase 后是 "livingroom",但前端 RoomId
                                    // 是 "living_room"(带下划线),字符串对不上小蓝点永不亮。
                                    // 同时也避开 derive Debug 的 snake_case 自动转换不可控
                                    selected_device_id(room, device)
                                };
                            }

                            // 发送交互状态到前端
                            if let Ok(ctx) = context.lock() {
                                // intentionally ignored: one-way UI notification, no recovery path
                                let _ = app.emit("interaction-state-changed", ctx.clone());
                            }

                            // 执行确认的动作
                            if let Some(action) = action {
                                log_info(&format!("[Gesture] 动作触发: {}", action));
                                execute_gesture_action(&action, &state, &app);

                                if let Ok(mut ctx) = context.lock() {
                                    ctx.last_action = Some(action.to_string());
                                }
                                // intentionally ignored: one-way UI notification, no recovery path
                                let _ = app.emit("gesture-action-confirmed", action.to_string());
                            }
                        }
                            Err(e) => {
                                // 手势识别失败（网络/解析/Python 错误）
                                log_warn(&format!("[Gesture] 识别失败: {}", e));
                                let failures = client.consecutive_failures();
                                if failures >= 5 {
                                    if let Ok(mut ctx) = context.lock() {
                                        ctx.health.python_ok = false;
                                    }
                                    // intentionally ignored: one-way UI notification, no recovery path
                                    let _ = app.emit("gesture-health-changed", GestureHealth {
                                        camera_ok: true,
                                        python_ok: false,
                                        camera_name: None,
                                    });
                                }
                                // 即使识别失败也发射交互状态，确保前端收到摄像头在线状态
                                if let Ok(ctx) = context.lock() {
                                    // intentionally ignored: one-way UI notification, no recovery path
                                    let _ = app.emit("interaction-state-changed", ctx.clone());
                                }
                            }
                        }
                    }
                } else {
                    // 无帧可用
                    if let Ok(mut ctx) = context.lock() {
                        ctx.is_tracking = false;
                        ctx.current_count = None;
                        ctx.dwell_progress = 0.0;
                        ctx.dwell_deadline_ms = None;
                    }
                }

                // 定期摄像头健康探测（仅在摄像头线程未运行时探测，避免覆盖其自身状态报告）
                if last_probe.elapsed() > Duration::from_millis(CAMERA_PROBE_INTERVAL_MS) {
                    last_probe = Instant::now();
                    // camera_thread_active=true: 摄像头线程正在运行，其错误处理是状态的权威来源
                    probe_camera_health(&context, true);
                }

                // 帧率控制
                let elapsed = loop_start.elapsed();
                let poll_interval = Duration::from_millis(GESTURE_POLL_INTERVAL_MS);
                if elapsed < poll_interval {
                    thread::sleep(poll_interval - elapsed);
                }
            }

            log_info("[Gesture] 手势识别线程退出");
        });

        Some(handle)
    }

    /// 停止手势识别
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.camera_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.gesture_thread.take() {
            let _ = handle.join();
        }

        log_info("[Gesture] 手势识别系统已停止");
    }

    /// 是否正在运行
    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// 获取交互上下文快照
    pub fn context_snapshot(&self) -> InteractionContext {
        self.context.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// 获取最新一帧 JPEG 字节（供前端 data URL 渲染使用）
    /// 返回 None 表示摄像头尚未产出第一帧（仍在初始化中）
    pub fn latest_frame(&self) -> Option<Vec<u8>> {
        self.frame_buffer.latest()
    }
}

/// 摄像头健康探测
/// `camera_thread_active`: 摄像头线程是否正在运行。
///   为 true 时跳过探测——摄像头线程自身的错误处理是 camera_ok 的权威来源。
fn probe_camera_health(context: &Arc<Mutex<InteractionContext>>, camera_thread_active: bool) {
    if camera_thread_active {
        return;
    }

    use nokhwa::query;

    let cameras = query(nokhwa::utils::ApiBackend::Auto);
    let found = cameras.map(|c| !c.is_empty()).unwrap_or(false);

    if let Ok(mut ctx) = context.lock() {
        ctx.health.camera_ok = found;
    }
}

/// 执行手势动作
fn execute_gesture_action(action: &GestureAction, state: &GlobalState, app: &AppHandle) {
    match action {
        GestureAction::SwitchDevice => {
            // 设备切换：只有前端 UI 需要更新，不修改 DeviceState
            // interaction-state-changed 事件已包含 selected_device 更新
        }
        GestureAction::ToggleDevice { room, device } => {
            // 关键修复:必须按"具体实例"(room, device)读取 is_on,
            // 旧版只看 living_room.* → 选客厅灯时拿客厅灯的 is_on
            // 决定开/关,但 bedroom 同类型设备状态被忽略,导致:
            //   - 卧室灯开 + 客厅灯关 → 选客厅灯 → 读 false → 发送 TurnOn
            //   - 卧室灯完全没受影响(因为 room=LivingRoom),本身没问题
            // 但若 room=All 时(没有 All 路径),会切两边。
            // 现在携带 room 维度,只切选中那一个,绝不连带。
            let is_on = {
                let guard = state.read().ok();
                guard.map(|g| {
                    let r = match room {
                        TargetRoom::LivingRoom => &g.living_room,
                        TargetRoom::Bedroom => &g.bedroom,
                        TargetRoom::All => &g.living_room, // 不会到这里
                    };
                    match device {
                        TargetDevice::Light => r.light.is_on,
                        TargetDevice::AirCondition => r.air_condition.is_on,
                        TargetDevice::Fan => r.fan.is_on,
                        TargetDevice::Curtain => r.curtain.is_open,
                        TargetDevice::All => false,
                    }
                }).unwrap_or(false)
            };

            let cmd_type = if is_on { CommandType::TurnOff } else { CommandType::TurnOn };
            let cmd = ParsedCommand {
                cmd_type,
                target: *device,
                value: None,
                room: *room,
            };

            if let Err(e) = control::execute_command(&cmd, state) {
                log_error(&format!("[Gesture] 执行失败: {:?}", e));
            }
        }
        GestureAction::AllOn => {
            let cmd = ParsedCommand {
                cmd_type: CommandType::TurnOn,
                target: TargetDevice::All,
                value: None,
                room: TargetRoom::All,
            };
            if let Err(e) = control::execute_command(&cmd, state) {
                log_error(&format!("[Gesture] 全部打开失败: {:?}", e));
            }
        }
        GestureAction::AllOff => {
            let cmd = ParsedCommand {
                cmd_type: CommandType::TurnOff,
                target: TargetDevice::All,
                value: None,
                room: TargetRoom::All,
            };
            if let Err(e) = control::execute_command(&cmd, state) {
                log_error(&format!("[Gesture] 全部关闭失败: {:?}", e));
            }
        }
        GestureAction::ToggleAll => {
            // 读全局状态 -> 决定 TurnOn 还是 TurnOff
            // 与 ToggleDevice (1028-1065) 的"读-分支-派发"模式一致
            // 任何设备开着 -> TurnOff (与当前 AllOff 行为一致)
            // 全部关闭 -> TurnOn (与当前 AllOn 行为一致)
            let any_on = is_any_device_on(state);
            let cmd_type = if any_on { CommandType::TurnOff } else { CommandType::TurnOn };
            let cmd = ParsedCommand {
                cmd_type,
                target: TargetDevice::All,
                value: None,
                room: TargetRoom::All,
            };
            if let Err(e) = control::execute_command(&cmd, state) {
                log_error(&format!("[Gesture] 全部切换失败: {:?}", e));
            }
        }
        GestureAction::Cancel => {
            // 取消不需要执行设备操作，FSM 会重置状态
        }
    }

    // 广播设备状态更新
    if let Ok(guard) = state.read() {
        // intentionally ignored: one-way UI notification, no recovery path
        let _ = app.emit("device-state-changed", guard.clone());
    }
}

/// 将 (room, device) 元组序列化为前端 `selected_device` 字符串 ID
/// 格式约定: "{room_snake}.{device_snake}" —— 必须与前端 `RoomId` ('living_room' / 'bedroom')
/// 和 DeviceType ('light' / 'fan' / 'ac' / 'curtain') 严格对齐。
///
/// 不能用 `format!("{:?}", ...).to_lowercase()` —— `TargetRoom::LivingRoom` 的 Debug
/// 输出是 `"LivingRoom"`,`to_lowercase()` 后是 `"livingroom"`(无下划线),
/// 与前端 `living_room` 不匹配,导致 isGestureSelected 永不命中、小蓝点全灭。
///
/// device 部分用 `{:?}` 是 OK 的 —— `TargetDevice` 的 Debug 输出就是
/// `"Light"`/`"AirCondition"`/`"Fan"`/`"Curtain"`,`to_lowercase()` 后正好与
/// 前端 `light`/`ac`/`fan`/`curtain` 一致。但为了一致性和未来安全(若 Debug 输出变了)
/// 全部走显式映射。
fn selected_device_id(room: TargetRoom, device: TargetDevice) -> String {
    let room_str = match room {
        TargetRoom::LivingRoom => "living_room",
        TargetRoom::Bedroom => "bedroom",
        TargetRoom::All => "all", // 理论上 GestureFsm 不会发出 All
    };
    let device_str = match device {
        TargetDevice::Light => "light",
        TargetDevice::AirCondition => "ac",
        TargetDevice::Fan => "fan",
        TargetDevice::Curtain => "curtain",
        TargetDevice::All => "all",
    };
    format!("{}.{}", room_str, device_str)
}

/// 检查当前全局状态中是否有任何设备处于"开启"状态
/// 用于 GestureAction::ToggleAll 决定下一次应 TurnOn 还是 TurnOff
///
/// 设备集合:living_room.{light, air_condition, fan, curtain}
///        + bedroom.{light, air_condition, fan, curtain}  (共 8 个实例)
///
/// "开启"的判定:
///   - light, air_condition, fan: 读取 `is_on` 字段
///   - curtain: 读取 `is_open` 字段 (这是窗帘的开关状态)
///
/// Returns:
///   - true: 至少一个设备处于开启状态
///   - false: 全部设备均关闭
///
/// 锁策略:函数从 state 读锁中提取所需字段后立即释放,然后计算结果。
/// 不要在持锁期间返回 —— 与 ToggleDevice 的 read-then-dispatch 模式一致。
fn is_any_device_on(state: &GlobalState) -> bool {
    // 任何设备"开启"的判定:
    //   - light / air_condition / fan: 读 is_on
    //   - curtain: 读 is_open (窗帘的开关字段)
    // 锁策略:read() 失败时保守返回 false,与 execute_gesture_action
    // 现有"读失败则按 off 处理"的策略一致——它会让 5 指走 TurnOn 分支,
    // 不会因一个不存在的错误而错关设备。
    let guard = match state.read() {
        Ok(g) => g,
        Err(_) => return false,
    };
    let g = &*guard;
    g.living_room.light.is_on
        || g.living_room.air_condition.is_on
        || g.living_room.fan.is_on
        || g.living_room.curtain.is_open
        || g.bedroom.light.is_on
        || g.bedroom.air_condition.is_on
        || g.bedroom.fan.is_on
        || g.bedroom.curtain.is_open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selected_device_id_alignment() {
        // 关键回归测试:ID 必须与前端 RoomId/DeviceType 字符串完全一致
        assert_eq!(
            selected_device_id(TargetRoom::LivingRoom, TargetDevice::Light),
            "living_room.light"
        );
        assert_eq!(
            selected_device_id(TargetRoom::Bedroom, TargetDevice::AirCondition),
            "bedroom.ac"
        );
        assert_eq!(
            selected_device_id(TargetRoom::LivingRoom, TargetDevice::Curtain),
            "living_room.curtain"
        );
    }

    /// 手指数量 0..5 必须严格映射到 6 个 GestureAction 变体之一。
    /// 这是 wire-format 重构(gesture: str -> count: u8)的安全网:
    /// 一旦 Python 端返回的数字与 Rust 端的 match 表不一致,这个测试会立刻失败。
    #[test]
    fn test_resolve_action_count_mapping() {
        let mut fsm = GestureFsm::new();

        // 0 指 → 取消(不依赖 FSM 状态)
        assert!(matches!(fsm.resolve_action(0), Some(GestureAction::Cancel)));

        // 1 指 → 切换到下一个设备(把 selected_device_index 推进 1)
        assert!(matches!(
            fsm.resolve_action(1),
            Some(GestureAction::SwitchDevice)
        ));
        assert_eq!(fsm.selected_device_index, 1);

        // 2 指 → 切换当前选中的具体设备实例
        // 刚推进过 1 次, 选中的是 (LivingRoom, AirCondition)
        let (room, device) = fsm.selected_device();
        assert!(matches!(
            fsm.resolve_action(2),
            Some(GestureAction::ToggleDevice { room, device })
        ));

        // 3 / 4 / 5 指 → 静态动作,不依赖 FSM 状态
        assert!(matches!(fsm.resolve_action(3), Some(GestureAction::AllOn)));
        assert!(matches!(fsm.resolve_action(4), Some(GestureAction::AllOff)));
        assert!(matches!(fsm.resolve_action(5), Some(GestureAction::ToggleAll)));

        // 越界 / 非法值 → None(防止误触)
        assert!(fsm.resolve_action(6).is_none());
        assert!(fsm.resolve_action(255).is_none());
    }

    /// `is_any_device_on` 必须是真正的 any-on 谓词,不能是 unimplemented!() 残留。
    /// 这个测试守护 5 指(ToggleAll)路径:之前 panic 会让工作线程崩溃,
    /// 整个手势识别就此失活。
    #[test]
    fn test_is_any_device_on() {
        use crate::state::{DeviceState, GlobalState};

        // 全新状态:所有设备都关 → false
        let state = GlobalState::new();
        assert!(!is_any_device_on(&state));

        // 单独翻转某个字段(覆盖 8 个设备实例中的每一个)
        let cases: [(&str, fn(&mut DeviceState) -> &mut bool); 8] = [
            ("living_room.light",          |s| &mut s.living_room.light.is_on),
            ("living_room.air_condition",  |s| &mut s.living_room.air_condition.is_on),
            ("living_room.fan",            |s| &mut s.living_room.fan.is_on),
            ("living_room.curtain",        |s| &mut s.living_room.curtain.is_open),
            ("bedroom.light",              |s| &mut s.bedroom.light.is_on),
            ("bedroom.air_condition",      |s| &mut s.bedroom.air_condition.is_on),
            ("bedroom.fan",                |s| &mut s.bedroom.fan.is_on),
            ("bedroom.curtain",            |s| &mut s.bedroom.curtain.is_open),
        ];
        for (label, set_on) in cases {
            let state = GlobalState::new();
            {
                let mut guard = state.write().expect("write lock");
                *set_on(&mut *guard) = true;
            }
            assert!(
                is_any_device_on(&state),
                "翻开了 {label} 后应当返回 true"
            );
        }
    }
}
