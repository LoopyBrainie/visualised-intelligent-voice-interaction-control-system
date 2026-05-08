import { batch, createEffect, createSignal, createUniqueId, onCleanup } from 'solid-js'
import { defaultFragment, FragmentFn } from '../lib/shaderUtils'

export interface LiquidGlassOptions {
  fragment?: FragmentFn
  canvasDPI?: number
  blur?: number | (() => number)
  contrast?: number
  brightness?: number
  saturate?: number
  snapshot?: () => HTMLCanvasElement | null
  snapshotVersion?: () => number
}

export function useLiquidGlass(options: LiquidGlassOptions = {}) {
  const fragment = options.fragment ?? defaultFragment
  const [canvasDPI, setCanvasDPI] = createSignal(
    options.canvasDPI ?? (typeof window !== 'undefined' ? window.devicePixelRatio : 1.5)
  )

  const filterId = createUniqueId()

  const [dimensions, setDimensions] = createSignal({ width: 0, height: 0 })

  let feImageEl: SVGFEImageElement | undefined
  let feDisplacementMapEl: SVGFEDisplacementMapElement | undefined
  let containerEl: HTMLElement | undefined
  let canvasEl: HTMLCanvasElement | undefined
  let canvasCtx: CanvasRenderingContext2D | undefined

  // Snapshot mode: private canvas for displaced result
  let displacedCanvas: HTMLCanvasElement | undefined
  let displacedCtx: CanvasRenderingContext2D | undefined
  let reusableImageData: ImageData | null = null
  // Two-canvas blur pipeline: putImageData bypasses CSS filters,
  // so we draw displaced→temp (with blur filter)→displaced
  let tempCanvas: HTMLCanvasElement | undefined
  let tempCtx: CanvasRenderingContext2D | undefined

  const [mousePos, setMousePos] = createSignal({ x: 0, y: 0 })

  const blurVal = options.blur ?? 0
  const blurAccessor: () => number = typeof blurVal === 'function'
    ? blurVal
    : () => blurVal
  // CSS 层仅做 1px 抗锯齿微模糊，深度模糊由 SVG feGaussianBlur 负责
  const cssBlurAccessor = () => Math.min(blurAccessor() * 0.15, 1)

  let rafId = 0
  let observer: ResizeObserver | undefined
  let prevBlobUrl: string | undefined

  function updateShader() {
    const w = dimensions().width
    const h = dimensions().height
    if (w === 0 || h === 0) return

    // === Snapshot mode: sample from frozen DOM snapshot ===
    const snap = options.snapshot?.()
    if (snap && snap.width > 0 && snap.height > 0 && displacedCanvas && displacedCtx) {
      const dpr = canvasDPI()
      const sw = Math.round(w * dpr)
      const sh = Math.round(h * dpr)

      displacedCanvas.width = sw
      displacedCanvas.height = sh

      // Viewport-relative crop from snapshot
      const rect = containerEl?.getBoundingClientRect()
      if (!rect) return
      const sx = Math.round(rect.left * dpr)
      const sy = Math.round(rect.top * dpr)

      // drawImage crop (GPU-accelerated)
      displacedCtx.drawImage(snap, sx, sy, sw, sh, 0, 0, sw, sh)

      // getImageData → per-pixel SDF displacement → putImageData
      if (!reusableImageData || reusableImageData.width !== sw || reusableImageData.height !== sh) {
        reusableImageData = displacedCtx.createImageData(sw, sh)
      }
      const srcData = displacedCtx.getImageData(0, 0, sw, sh)
      const dstData = reusableImageData
      const src = srcData.data
      const dst = dstData.data

      let mouseUsed = false
      const currentMouse = mousePos()
      const mouseProxy = new Proxy(currentMouse, {
        get(target, prop) {
          mouseUsed = true
          return target[prop as keyof typeof target]
        },
      })

      for (let i = 0; i < src.length; i += 4) {
        const px = (i / 4) % sw
        const py = Math.floor(i / 4 / sw)
        const uvX = px / (w * dpr)
        const uvY = py / (h * dpr)
        const rawPos = fragment(
          { x: uvX, y: uvY },
          mouseProxy,
          { width: w, height: h }
        )
        // Sample source at displaced position
        const sampleX = Math.round(rawPos.x * w * dpr)
        const sampleY = Math.round(rawPos.y * h * dpr)
        const clampedX = Math.max(0, Math.min(sw - 1, sampleX))
        const clampedY = Math.max(0, Math.min(sh - 1, sampleY))
        const srcIdx = (clampedY * sw + clampedX) * 4
        dst[i] = src[srcIdx]
        dst[i + 1] = src[srcIdx + 1]
        dst[i + 2] = src[srcIdx + 2]
        dst[i + 3] = src[srcIdx + 3]
      }

      // putImageData bypasses CSS filters (per Canvas spec).
      // Two-canvas pipeline: displaced→temp (with blur filter)→displaced
      displacedCtx.putImageData(dstData, 0, 0)

      const blur = blurAccessor()
      if (blur > 0) {
        if (!tempCanvas) {
          tempCanvas = document.createElement('canvas')
          tempCtx = tempCanvas.getContext('2d')!
        }
        tempCanvas.width = sw
        tempCanvas.height = sh
        tempCtx!.filter = `blur(${blur}px)`
        tempCtx!.drawImage(displacedCanvas, 0, 0)
        displacedCtx.clearRect(0, 0, sw, sh)
        displacedCtx.drawImage(tempCanvas, 0, 0)
      }

      return mouseUsed
    }

    // === Fallback: SVG displacement map mode ===
    if (!canvasEl || !canvasCtx || !feImageEl || !feDisplacementMapEl) return

    // Canvas 尺寸精确匹配元素（无 padding），防止位移采样越界
    const dpi = canvasDPI()
    const cw = Math.round(w * dpi)
    const ch = Math.round(h * dpi)

    canvasEl.width = cw
    canvasEl.height = ch

    let mouseUsed = false
    const currentMouse = mousePos()
    const mouseProxy = new Proxy(currentMouse, {
      get(target, prop) {
        mouseUsed = true
        return target[prop as keyof typeof target]
      },
    })

    const data = new Uint8ClampedArray(cw * ch * 4)
    let maxScale = 0
    const rawValues: number[] = []

    for (let i = 0; i < data.length; i += 4) {
      const x = (i / 4) % cw
      const y = Math.floor(i / 4 / cw)
      // UV 映射：canvas 像素 → [0,1] UV 坐标
      const uvX = x / (w * dpi)
      const uvY = y / (h * dpi)
      const rawPos = fragment(
        { x: uvX, y: uvY },
        mouseProxy,
        { width: w, height: h }
      )
      // 钳位 fragment 输出到 [0,1]
      const pos = {
        x: Math.max(0, Math.min(1, rawPos.x)),
        y: Math.max(0, Math.min(1, rawPos.y)),
      }
      const dx = pos.x * (w * dpi) - x
      const dy = pos.y * (h * dpi) - y
      maxScale = Math.max(maxScale, Math.abs(dx), Math.abs(dy))
      rawValues.push(dx, dy)
    }

    // Guard: maxScale=0 → no displacement → identity map (all 0.5)
    const normScale = maxScale > 0 ? 1.2 * maxScale : 1
    let index = 0
    for (let i = 0; i < data.length; i += 4) {
      const r = rawValues[index++] / normScale + 0.5
      const g = rawValues[index++] / normScale + 0.5
      data[i] = r * 255
      data[i + 1] = g * 255
      data[i + 2] = 0
      data[i + 3] = 255
    }

    canvasCtx.putImageData(new ImageData(data, cw, ch), 0, 0)
    // toBlob 避免 toDataURL 的 base64 编码开销
    canvasEl.toBlob((blob) => {
      if (blob && feImageEl) {
        if (prevBlobUrl) URL.revokeObjectURL(prevBlobUrl)
        prevBlobUrl = URL.createObjectURL(blob)
        feImageEl.setAttributeNS('http://www.w3.org/1999/xlink', 'href', prevBlobUrl)
      }
    })
    // SVG feDisplacementMap: P' = P + scale × (C - 0.5)
    // scale = maxScale / dpi 使位移量精确还原（无 padding 偏移）
    feDisplacementMapEl.setAttribute('scale', (maxScale / dpi).toString())

    return mouseUsed
  }

  function scheduleUpdate() {
    if (rafId) cancelAnimationFrame(rafId)
    rafId = requestAnimationFrame(() => {
      rafId = 0
      updateShader()
    })
  }

  const containerRef = (el: HTMLElement) => {
    containerEl = el

    // Mouse tracking
    const handleMouseMove = (e: MouseEvent) => {
      const rect = el.getBoundingClientRect()
      batch(() => {
        setMousePos({
          x: (e.clientX - rect.left) / rect.width,
          y: (e.clientY - rect.top) / rect.height,
        })
      })
      // Only re-render if fragment uses mouse (coalesced via rAF)
      scheduleUpdate()
    }
    el.addEventListener('mousemove', handleMouseMove)

    // ResizeObserver for responsive dimensions
    observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const box = entry.borderBoxSize?.[0]
        if (box) {
          setDimensions({ width: Math.round(box.inlineSize), height: Math.round(box.blockSize) })
        } else {
          setDimensions({ width: el.offsetWidth, height: el.offsetHeight })
        }
        scheduleUpdate()
      }
    })
    observer.observe(el, { box: 'border-box' })

    // Listen for DPI changes (e.g. drag window between monitors)
    let dprQuery: MediaQueryList | undefined
    let handleDpiChange: (() => void) | undefined
    if (options.canvasDPI == null && typeof window !== 'undefined') {
      dprQuery = window.matchMedia(
        `(resolution: ${window.devicePixelRatio}dppx)`
      )
      handleDpiChange = () => {
        setCanvasDPI(window.devicePixelRatio)
        scheduleUpdate()
      }
      dprQuery.addEventListener('change', handleDpiChange)
    }

    onCleanup(() => {
      el.removeEventListener('mousemove', handleMouseMove)
      observer?.disconnect()
      if (rafId) cancelAnimationFrame(rafId)
      if (prevBlobUrl) URL.revokeObjectURL(prevBlobUrl)
      if (dprQuery && handleDpiChange) {
        dprQuery.removeEventListener('change', handleDpiChange)
      }
    })
  }

  const canvasRef = (el: HTMLCanvasElement) => {
    canvasEl = el
    canvasCtx = el.getContext('2d')!
    // Trigger initial render if dimensions already known
    scheduleUpdate()
  }

  // SVG element refs
  const feImageRef = (el: SVGFEImageElement) => {
    feImageEl = el
  }

  const feDisplacementMapRef = (el: SVGFEDisplacementMapElement) => {
    feDisplacementMapEl = el
    scheduleUpdate()
  }

  // Displaced canvas ref (snapshot mode)
  const displacedCanvasRef = (el: HTMLCanvasElement) => {
    displacedCanvas = el
    displacedCtx = el.getContext('2d')!
    scheduleUpdate()
  }

  // Re-render when snapshot changes
  if (options.snapshotVersion) {
    createEffect(() => {
      options.snapshotVersion!()
      scheduleUpdate()
    })
  }

  // Re-render when snapshot accessor transitions from null to available
  if (options.snapshot) {
    createEffect(() => {
      options.snapshot!()
      scheduleUpdate()
    })
  }

  return {
    containerRef,
    canvasRef,
    feImageRef,
    feDisplacementMapRef,
    displacedCanvasRef,
    filterId,
    dimensions,
    blurAccessor,
  }
}
