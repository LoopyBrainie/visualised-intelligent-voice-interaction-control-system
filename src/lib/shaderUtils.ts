export type TextureCoord = { type: 't'; x: number; y: number }

export type FragmentFn = (
  uv: { x: number; y: number },
  mouse: { x: number; y: number },
  size: { width: number; height: number }
) => TextureCoord

export function smoothStep(a: number, b: number, t: number): number {
  t = Math.max(0, Math.min(1, (t - a) / (b - a)))
  return t * t * (3 - 2 * t)
}

export function length(x: number, y: number): number {
  return Math.sqrt(x * x + y * y)
}

export function roundedRectSDF(
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number
): number {
  const qx = Math.abs(x) - width + radius
  const qy = Math.abs(y) - height + radius
  return (
    Math.min(Math.max(qx, qy), 0) +
    length(Math.max(qx, 0), Math.max(qy, 0)) -
    radius
  )
}

export function texture(x: number, y: number): TextureCoord {
  return { type: 't', x, y }
}

export const defaultFragment: FragmentFn = (uv, _mouse, size) => {
  const px = uv.x * size.width
  const py = uv.y * size.height
  const hw = size.width / 2
  const hh = size.height / 2
  const radius = 16
  const edgeWidth = 8
  // SDF in pixel space: negative inside, positive outside
  const d = roundedRectSDF(px - hw, py - hh, hw, hh, radius)
  // edgeFactor: 0 deep inside (transmission), 1 at/near edge (refraction)
  const edgeFactor = smoothStep(-edgeWidth, 0, d)
  const scaled = 1 - edgeFactor * 0.12
  const cx = (px - hw) * scaled + hw
  const cy = (py - hh) * scaled + hh
  return texture(cx / size.width, cy / size.height)
}
