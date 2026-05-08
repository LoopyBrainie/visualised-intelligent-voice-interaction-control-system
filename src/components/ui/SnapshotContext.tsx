import { createContext, createSignal, onCleanup, onMount, ParentComponent, useContext } from 'solid-js'
import { domToCanvas } from 'modern-screenshot'

interface SnapshotContextValue {
  canvas: () => HTMLCanvasElement | null
  isReady: () => boolean
  snapshotVersion: () => number
  notifyPanelOpen: () => Promise<void>
  notifyPanelClose: () => void
}

const SnapshotContext = createContext<SnapshotContextValue>()

export const SnapshotProvider: ParentComponent = (props) => {
  let snapshotCanvas!: HTMLCanvasElement
  const [isReady, setIsReady] = createSignal(false)
  const [snapshotVersion, setSnapshotVersion] = createSignal(0)
  const [isLocked, setIsLocked] = createSignal(false)

  let captureInFlight = false
  let debounceTimer: ReturnType<typeof setTimeout> | undefined

  async function captureSnapshot() {
    if (isLocked() || captureInFlight) return
    captureInFlight = true
    const root = document.getElementById('root')
    if (!root) { captureInFlight = false; return }

    const dpr = window.devicePixelRatio

    try {
      // 超时保护：domToCanvas 可能挂起，防止 captureInFlight 永久锁定
      const capturePromise = domToCanvas(root, {
        scale: dpr,
        backgroundColor: null,
        // 排除 glass overlay 高光层和快照 canvas 自身（避免递归捕获）
        filter: (node) => {
          if (node instanceof HTMLElement && node.hasAttribute('data-glass-overlay')) return false
          if (node instanceof HTMLElement && node.hasAttribute('data-snapshot-canvas')) return false
          return true
        },
      })
      const timeoutPromise = new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error('domToCanvas timeout')), 5000)
      )
      const canvas = await Promise.race([capturePromise, timeoutPromise])

      // 拷贝到共享 Canvas
      snapshotCanvas.width = canvas.width
      snapshotCanvas.height = canvas.height
      const ctx = snapshotCanvas.getContext('2d')!
      ctx.clearRect(0, 0, canvas.width, canvas.height)
      ctx.drawImage(canvas, 0, 0)

      setSnapshotVersion(v => v + 1)
    } catch (err) {
      console.warn('[SnapshotContext] capture failed:', err)
    } finally {
      captureInFlight = false
    }
  }

  function scheduleCapture() {
    if (isLocked() || captureInFlight) return
    clearTimeout(debounceTimer)
    debounceTimer = setTimeout(captureSnapshot, 300)
  }

  onMount(() => {
    setIsReady(true)
    captureSnapshot()

    const dpr = window.devicePixelRatio
    const dprQuery = window.matchMedia(`(resolution: ${dpr}dppx)`)
    const handleDpiChange = () => {
      if (!isLocked()) captureSnapshot()
    }
    dprQuery.addEventListener('change', handleDpiChange)

    // Re-capture when DOM changes (debounced)
    const root = document.getElementById('root')
    let mutationObserver: MutationObserver | undefined
    if (root) {
      mutationObserver = new MutationObserver(scheduleCapture)
      mutationObserver.observe(root, {
        childList: true,
        subtree: true,
        attributes: true,
        characterData: true,
      })
    }

    // Re-capture on window resize
    const handleResize = () => scheduleCapture()
    window.addEventListener('resize', handleResize)

    onCleanup(() => {
      dprQuery.removeEventListener('change', handleDpiChange)
      mutationObserver?.disconnect()
      clearTimeout(debounceTimer)
      window.removeEventListener('resize', handleResize)
    })
  })

  return (
    <SnapshotContext.Provider value={{
      canvas: () => snapshotCanvas,
      isReady,
      snapshotVersion,
      notifyPanelOpen: async () => {
        await captureSnapshot()
        setIsLocked(true)
      },
      notifyPanelClose: () => {
        setIsLocked(false)
      },
    }}>
      <canvas ref={snapshotCanvas} data-snapshot-canvas style={{ display: 'none' }} />
      {props.children}
    </SnapshotContext.Provider>
  )
}

export function useSnapshot() {
  const ctx = useContext(SnapshotContext)
  if (!ctx) throw new Error('useSnapshot must be used within SnapshotProvider')
  return ctx
}
