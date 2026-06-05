import { onCleanup, createSignal, createEffect } from 'solid-js';
import { subscribeSpectrum, unsubscribeSpectrum, createMockSpectrumData } from '../lib/spectrum';
import { interactionContext, type Landmark } from '../store/interactionStore';
import { typedInvoke } from '@/errors';
import { useResponsiveLayout } from '../hooks/useResponsiveLayout';

interface SpectrumPanelProps {
  mode: 'voice' | 'gesture';
}

// Tauri 注入的 runtime 标识;不存在时处于 Vite 浏览器模式
// (Tauri 模式通过 get_camera_frame 命令拉取最新帧 + Blob URL 渲染;
// 浏览器模式回退到 getUserMedia)
const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// MediaPipe 手部骨架连接关系 (21 关键点)
const HAND_CONNECTIONS: [number, number][] = [
  [0, 1], [1, 2], [2, 3], [3, 4],       // Thumb
  [0, 5], [5, 6], [6, 7], [7, 8],       // Index
  [0, 9], [9, 10], [10, 11], [11, 12],  // Middle
  [0, 13], [13, 14], [14, 15], [15, 16],// Ring
  [0, 17], [17, 18], [18, 19], [19, 20],// Pinky
  [5, 9], [9, 13], [13, 17],            // Palm
];

const FINGERTIP_INDICES = [4, 8, 12, 16, 20];

function drawSkeleton(
  ctx: CanvasRenderingContext2D,
  landmarks: Landmark[],
  w: number,
  h: number,
) {
  // Draw connections
  ctx.strokeStyle = 'rgba(0, 113, 227, 0.7)';
  ctx.lineWidth = 2;
  for (const [i, j] of HAND_CONNECTIONS) {
    const p1 = landmarks[i];
    
    const p2 = landmarks[j];
    ctx.beginPath();
    ctx.moveTo(p1.x * w, p1.y * h);
    ctx.lineTo(p2.x * w, p2.y * h);
    ctx.stroke();
  }

  // Draw landmark points
  ctx.fillStyle = '#ffffff';
  ctx.strokeStyle = 'rgba(0, 113, 227, 0.9)';
  ctx.lineWidth = 1.5;
  for (const lm of landmarks) {
    const x = lm.x * w;
    const y = lm.y * h;
    ctx.beginPath();
    ctx.arc(x, y, 3, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }

  // Highlight fingertips
  ctx.fillStyle = 'rgba(0, 113, 227, 0.9)';
  for (const idx of FINGERTIP_INDICES) {
    const lm = landmarks[idx];
    ctx.beginPath();
    ctx.arc(lm.x * w, lm.y * h, 4, 0, Math.PI * 2);
    ctx.fill();
  }
}

export function SpectrumPanel(props: SpectrumPanelProps) {
  const layout = useResponsiveLayout();
  // Bar count drops with screen real estate: 16 at very short windows,
  // 24 at narrow, 32 at standard and above.
  const barCount = () => {
    if (layout.heightTier === 'h0') return 16;
    if (layout.widthTier === 'n') return 24;
    return 32;
  };
  const [spectrumData, setSpectrumData] = createSignal<number[]>(Array(barCount()).fill(0.05));
  const [isListening, setIsListening] = createSignal(false);

  let cameraImg: HTMLImageElement | undefined;
  let cameraVideo: HTMLVideoElement | undefined;
  let canvasEl: HTMLCanvasElement | undefined;
  let canvasCtx: CanvasRenderingContext2D | undefined;
  let mockInterval: number;
  let mediaStream: MediaStream | null = null;
  let rafId: number | undefined;

  /**
   * 计算 object-contain 后**实际渲染**的图像矩形(剔除 letterbox 黑边)
   * - <img> 用 naturalWidth/Height;<video> 用 videoWidth/Height
   * - 未加载时这些值为 0,返回 null
   *
   * object-contain 规则: 按比例缩放,长边填满容器,短边方向留黑边
   *   - imgAspect > boxAspect (图像更宽): 宽度填满,上下黑边
   *   - imgAspect < boxAspect (图像更高): 高度填满,左右黑边
   *
   * 必须用这个矩形驱动 canvas —— 否则 skeleton landmarks [0,1]
   * 会拉伸到 letterbox 黑边上(典型症状: 640x480 4:3 摄像头在
   * 16:9 预览框里,骨架横向被拉宽 33%)
   */
  function imageRect() {
    const el = isTauri ? cameraImg : cameraVideo;
    if (!el) return null;
    const box = el.getBoundingClientRect();
    const nw =
      (el as HTMLImageElement).naturalWidth ||
      (el as HTMLVideoElement).videoWidth;
    const nh =
      (el as HTMLImageElement).naturalHeight ||
      (el as HTMLVideoElement).videoHeight;
    if (!nw || !nh) return null;
    const imgAspect = nw / nh;
    const boxAspect = box.width / box.height;
    if (imgAspect > boxAspect) {
      const renderH = box.width / imgAspect;
      return { x: 0, y: (box.height - renderH) / 2, w: box.width, h: renderH };
    }
    const renderW = box.height * imgAspect;
    return { x: (box.width - renderW) / 2, y: 0, w: renderW, h: box.height };
  }

  function syncCanvasSize() {
    const rect = imageRect();
    if (!rect || !canvasEl) return;
    const dpr = window.devicePixelRatio || 1;
    // 关键: canvas 钉在 object-contain 后的实际图像矩形上,
    // 而不是整个预览框 — letterbox 不属于图像坐标系
    canvasEl.style.left = `${rect.x}px`;
    canvasEl.style.top = `${rect.y}px`;
    canvasEl.style.width = `${rect.w}px`;
    canvasEl.style.height = `${rect.h}px`;
    canvasEl.width = Math.round(rect.w * dpr);
    canvasEl.height = Math.round(rect.h * dpr);
    canvasCtx = canvasEl.getContext('2d')!;
    // 用 setTransform 而非 scale,避免 ResizeObserver 反复触发时
    // 累积缩放矩阵(每次都用绝对值重置)
    canvasCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  function drawFrame() {
    if (!canvasEl || !canvasCtx) return;
    // canvas 已被 syncCanvasSize 定位在图像矩形上,
    // 直接用 clientWidth/clientHeight 作为骨架绘制坐标系
    const w = canvasEl.clientWidth;
    const h = canvasEl.clientHeight;
    canvasCtx.clearRect(0, 0, w, h);
    const ctx = interactionContext();
    if (ctx.landmarks?.length) {
      drawSkeleton(canvasCtx, ctx.landmarks, w, h);
    }
  }

  // 模式变化时启动/停止相应的预览管道。
  // 注意: 必须用 createEffect 而非 onMount —
  // SpectrumPanel 实例不会随模式变化而卸载,onMount 只触发一次
  // (原 bug: 应用初始为语音模式时切换到手势,摄像头预览永不启动)
  createEffect(() => {
    // 每次 effect 重跑前,先清理上一轮的资源
    onCleanup(() => {
      unsubscribeSpectrum();
      clearInterval(mockInterval);
      if (rafId !== undefined) {
        cancelAnimationFrame(rafId);
        rafId = undefined;
      }
      if (mediaStream) {
        mediaStream.getTracks().forEach((t) => t.stop());
        mediaStream = null;
      }
    });

    if (props.mode === 'voice') {
      try {
        subscribeSpectrum((data) => {
          setSpectrumData(data);
          if (!isListening()) setIsListening(true);
        });
      } catch {
        startMockAnimation();
      }
    } else if (isTauri) {
      startTauriCameraPreview();
    } else {
      startBrowserCameraPreview();
    }
  });

  function startMockAnimation() {
    mockInterval = window.setInterval(() => {
      setSpectrumData(createMockSpectrumData(32));
    }, 33);
  }

  function startTauriCameraPreview() {
    if (canvasEl?.parentElement) {
      const observer = new ResizeObserver(syncCanvasSize);
      observer.observe(canvasEl.parentElement);
      onCleanup(() => observer.disconnect());
    }
    syncCanvasSize();

    // 异步轮询: 每次 typedInvoke 拿最新一帧 JPEG bytes, 包成 Blob URL
    // 赋给 <img>.src。Blob URL 比 data:URL 内存效率高(无需 base64),
    // 且 webview 原生支持,不需要任何 scheme 注册。
    // 旧版 camera:// 自定义协议在 Tauri 2 + WebView2 上 WebView2 自身
    // 不识别该 scheme, 抛 ERR_UNKNOWN_URL_SCHEME (请求在到达 Tauri
    // 主机进程前就被 webview 拒绝), 完全没法用。
    let cancelled = false;
    onCleanup(() => { cancelled = true; });

    let currentBlobUrl: string | null = null;
    onCleanup(() => {
      if (currentBlobUrl) URL.revokeObjectURL(currentBlobUrl);
    });

    async function pollNextFrame() {
      if (cancelled) return;
      const result = await typedInvoke<number[] | null>('get_camera_frame');
      if (cancelled || !cameraImg) return;

      if (result.ok && result.value && result.value.length > 0) {
        const blob = new Blob([new Uint8Array(result.value)], { type: 'image/jpeg' });
        const url = URL.createObjectURL(blob);
        const prev = currentBlobUrl;
        currentBlobUrl = url;
        cameraImg.src = url;
        // 上一帧 Blob URL 立即释放 — <img>.src 已切换,旧 URL 不再被引用
        if (prev) URL.revokeObjectURL(prev);
        drawFrame();
      }
      // 摄像头尚未产出第一帧(None)或错误时,33ms 后重试
      rafId = window.setTimeout(pollNextFrame, 33);
    }

    cameraImg.onload = () => {
      // naturalWidth 此时才非零,需要重新计算图像矩形
      // (首次 syncCanvasSize 在第一帧到达前 naturalWidth=0 → 跳过)
      syncCanvasSize();
      drawFrame();
    };

    // Kick off first request
    pollNextFrame();
  }

  /**
   * 浏览器模式 (Vite :3000 无 Tauri runtime) 回退:
   * 使用 getUserMedia 直接读取本地摄像头,通过 <video> 元素显示。
   * 骨架覆盖层照常基于 interactionContext().landmarks 绘制。
   * 注意: 此路径不连接 Python 手势识别,landmarks 始终为空,
   * 仅用于浏览器模式下预览摄像头 UI 与验证 getUserMedia 权限。
   */
  function startBrowserCameraPreview() {
    if (canvasEl?.parentElement) {
      const observer = new ResizeObserver(syncCanvasSize);
      observer.observe(canvasEl.parentElement);
      onCleanup(() => observer.disconnect());
    }
    syncCanvasSize();

    if (!navigator.mediaDevices?.getUserMedia) {
      console.warn('[SpectrumPanel] 浏览器不支持 getUserMedia,无法预览摄像头');
      return;
    }

    navigator.mediaDevices
      .getUserMedia({ video: { width: 640, height: 480 }, audio: false })
      .then((stream) => {
        mediaStream = stream;
        if (cameraVideo) {
          cameraVideo.srcObject = stream;
          // videoWidth/videoHeight 在元数据加载完成后才可用,
          // 必须重新计算图像矩形 — 否则 canvas 仍按 videoWidth=0 跳过
          cameraVideo.addEventListener(
            'loadedmetadata',
            () => syncCanvasSize(),
            { once: true },
          );
          cameraVideo.play().catch(() => {
            /* autoplay 阻止 — 已被 muted 绕过 */
          });
        }
        const tick = () => {
          if (!cameraVideo || cameraVideo.paused || cameraVideo.ended) {
            rafId = requestAnimationFrame(tick);
            return;
          }
          drawFrame();
          rafId = requestAnimationFrame(tick);
        };
        rafId = requestAnimationFrame(tick);
      })
      .catch((err) => {
        console.warn(`[SpectrumPanel] getUserMedia 失败: ${err.message ?? err}`);
      });
  }

  return (
    <div class="w-full h-full flex items-center justify-center px-xl">
      {props.mode === 'voice' ? (
        /* Spectrum bars — full width, centered, rounded caps */
        <div class="w-full h-full flex items-end justify-center gap-xs py-md">
          {Array.from({ length: barCount() }, (_, i) => {
            const scale = spectrumData()[i] || 0.05;
            const isActive = scale > 0.1;
            return (
              <div
                class="flex-1 max-w-[var(--spectrum-bar-max-width)] rounded-t-full"
                style={{
                  height: '80%',
                  transform: `scaleY(${scale})`,
                  'transform-origin': 'bottom',
                  background: isActive ? 'var(--color-accent)' : 'var(--color-border)',
                  transition: 'transform 150ms cubic-bezier(0.25, 1, 0.5, 1)',
                }}
              />
            );
          })}
        </div>
      ) : (
        /* Camera preview with skeleton overlay */
        <div class="w-full h-full flex items-center justify-center rounded-md overflow-hidden bg-black/5 relative">
          {isTauri ? (
            <img
              ref={(el) => { cameraImg = el; }}
              alt="摄像头预览"
              class="w-full h-full object-contain"
              style={{ transform: 'scaleX(-1)' }}
            />
          ) : (
            <video
              ref={(el) => { cameraVideo = el; }}
              autoplay
              muted
              playsinline
              class="w-full h-full object-contain"
              style={{ transform: 'scaleX(-1)' }}
            />
          )}
          <canvas
            ref={(el) => { canvasEl = el; }}
            class="absolute inset-0 w-full h-full pointer-events-none"
            style={{ transform: 'scaleX(-1)' }}
          />
        </div>
      )}
    </div>
  );
}
