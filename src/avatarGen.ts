/** Local multi-style avatar generator (SVG data URLs, no network). */

export const AVATAR_COLORS = [
  '#0d9488',
  '#0369a1',
  '#b45309',
  '#be123c',
  '#4d7c0f',
  '#6d28d9',
  '#0f766e',
  '#1e3a5f',
  '#ea580c',
  '#db2777',
  '#7c3aed',
  '#0891b2',
  '#65a30d',
  '#c2410c',
  '#4338ca',
  '#0e7490',
  '#be185d',
  '#15803d',
  '#b45309',
  '#334155',
]

export type AvatarStyleId =
  | 'solid'
  | 'gradient'
  | 'geo'
  | 'mosaic'
  | 'pixel'
  | 'rings'
  | 'wave'
  | 'blob'

export interface AvatarStyleMeta {
  id: AvatarStyleId
  label: string
}

export const AVATAR_STYLES: AvatarStyleMeta[] = [
  { id: 'solid', label: '纯色' },
  { id: 'gradient', label: '渐变' },
  { id: 'geo', label: '几何' },
  { id: 'mosaic', label: '马赛克' },
  { id: 'pixel', label: '像素' },
  { id: 'rings', label: '圆环' },
  { id: 'wave', label: '波纹' },
  { id: 'blob', label: '色块' },
]

function hashSeed(seed: string): number {
  let h = 2166136261
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return h >>> 0
}

function mulberry32(seed: number) {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

function pick<T>(rand: () => number, arr: readonly T[]): T {
  return arr[Math.floor(rand() * arr.length)]!
}

function svgUrl(svg: string): string {
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg.replace(/\s+/g, ' ').trim())}`
}

function gradientSvg(c1: string, c2: string, angle: number) {
  const id = `g${Math.abs(hashSeed(c1 + c2 + angle)).toString(36)}`
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">
    <defs><linearGradient id="${id}" gradientTransform="rotate(${angle})">
      <stop offset="0%" stop-color="${c1}"/><stop offset="100%" stop-color="${c2}"/>
    </linearGradient></defs>
    <circle cx="40" cy="40" r="40" fill="url(#${id})"/>
  </svg>`)
}

function geoSvg(rand: () => number) {
  const bg = pick(rand, AVATAR_COLORS)
  const shapes: string[] = []
  const n = 4 + Math.floor(rand() * 5)
  for (let i = 0; i < n; i++) {
    const c = pick(rand, AVATAR_COLORS)
    const op = (0.35 + rand() * 0.55).toFixed(2)
    if (rand() > 0.45) {
      const x = (rand() * 80).toFixed(1)
      const y = (rand() * 80).toFixed(1)
      const r = (8 + rand() * 28).toFixed(1)
      shapes.push(`<circle cx="${x}" cy="${y}" r="${r}" fill="${c}" opacity="${op}"/>`)
    } else {
      const x1 = (rand() * 80).toFixed(1)
      const y1 = (rand() * 80).toFixed(1)
      const x2 = (rand() * 80).toFixed(1)
      const y2 = (rand() * 80).toFixed(1)
      const x3 = (rand() * 80).toFixed(1)
      const y3 = (rand() * 80).toFixed(1)
      shapes.push(
        `<polygon points="${x1},${y1} ${x2},${y2} ${x3},${y3}" fill="${c}" opacity="${op}"/>`,
      )
    }
  }
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">
    <circle cx="40" cy="40" r="40" fill="${bg}"/>
    <clipPath id="clip"><circle cx="40" cy="40" r="40"/></clipPath>
    <g clip-path="url(#clip)">${shapes.join('')}</g>
  </svg>`)
}

function mosaicSvg(rand: () => number) {
  const cells = 4 + Math.floor(rand() * 3)
  const size = 80 / cells
  const rects: string[] = []
  for (let y = 0; y < cells; y++) {
    for (let x = 0; x < cells; x++) {
      const c = pick(rand, AVATAR_COLORS)
      rects.push(
        `<rect x="${(x * size).toFixed(2)}" y="${(y * size).toFixed(2)}" width="${size + 0.4}" height="${size + 0.4}" fill="${c}"/>`,
      )
    }
  }
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">
    <clipPath id="clip"><circle cx="40" cy="40" r="40"/></clipPath>
    <g clip-path="url(#clip)">${rects.join('')}</g>
  </svg>`)
}

function pixelSvg(rand: () => number) {
  const grid = 8
  const size = 80 / grid
  const bg = pick(rand, AVATAR_COLORS)
  const ink = pick(rand, AVATAR_COLORS)
  const accent = pick(rand, AVATAR_COLORS)
  const rects: string[] = [`<rect width="80" height="80" fill="${bg}"/>`]
  // Symmetrical pixel creature / pattern
  for (let y = 1; y < grid - 1; y++) {
    for (let x = 1; x < Math.ceil(grid / 2); x++) {
      if (rand() > 0.55) continue
      const c = rand() > 0.7 ? accent : ink
      const mirror = grid - 1 - x
      for (const px of [x, mirror]) {
        rects.push(
          `<rect x="${px * size}" y="${y * size}" width="${size}" height="${size}" fill="${c}"/>`,
        )
      }
    }
  }
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">
    <clipPath id="clip"><circle cx="40" cy="40" r="40"/></clipPath>
    <g clip-path="url(#clip)">${rects.join('')}</g>
  </svg>`)
}

function ringsSvg(rand: () => number) {
  const bg = pick(rand, AVATAR_COLORS)
  const rings: string[] = [`<circle cx="40" cy="40" r="40" fill="${bg}"/>`]
  const n = 3 + Math.floor(rand() * 4)
  for (let i = 0; i < n; i++) {
    const r = 12 + i * (22 / n) + rand() * 4
    const c = pick(rand, AVATAR_COLORS)
    const w = 3 + rand() * 5
    rings.push(
      `<circle cx="40" cy="40" r="${r.toFixed(1)}" fill="none" stroke="${c}" stroke-width="${w.toFixed(1)}" opacity="0.85"/>`,
    )
  }
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">${rings.join('')}</svg>`)
}

function waveSvg(rand: () => number) {
  const c1 = pick(rand, AVATAR_COLORS)
  const c2 = pick(rand, AVATAR_COLORS)
  const c3 = pick(rand, AVATAR_COLORS)
  const amp = 6 + rand() * 10
  const paths: string[] = [`<rect width="80" height="80" fill="${c1}"/>`]
  for (let i = 0; i < 4; i++) {
    const y0 = 18 + i * 16
    const phase = rand() * Math.PI * 2
    let d = `M0 ${y0}`
    for (let x = 0; x <= 80; x += 8) {
      const y = y0 + Math.sin(x / 12 + phase) * amp
      d += ` L${x} ${y.toFixed(1)}`
    }
    d += ` L80 80 L0 80 Z`
    const c = i % 2 === 0 ? c2 : c3
    paths.push(`<path d="${d}" fill="${c}" opacity="${(0.45 + i * 0.12).toFixed(2)}"/>`)
  }
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">
    <clipPath id="clip"><circle cx="40" cy="40" r="40"/></clipPath>
    <g clip-path="url(#clip)">${paths.join('')}</g>
  </svg>`)
}

function blobSvg(rand: () => number) {
  const bg = pick(rand, AVATAR_COLORS)
  const blobs: string[] = [`<circle cx="40" cy="40" r="40" fill="${bg}"/>`]
  const n = 3 + Math.floor(rand() * 4)
  for (let i = 0; i < n; i++) {
    const c = pick(rand, AVATAR_COLORS)
    const cx = 15 + rand() * 50
    const cy = 15 + rand() * 50
    const rx = 12 + rand() * 22
    const ry = 12 + rand() * 22
    const rot = Math.floor(rand() * 360)
    blobs.push(
      `<ellipse cx="${cx.toFixed(1)}" cy="${cy.toFixed(1)}" rx="${rx.toFixed(1)}" ry="${ry.toFixed(1)}" fill="${c}" opacity="0.75" transform="rotate(${rot} ${cx.toFixed(1)} ${cy.toFixed(1)})"/>`,
    )
  }
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">
    <clipPath id="clip"><circle cx="40" cy="40" r="40"/></clipPath>
    <g clip-path="url(#clip)">${blobs.join('')}</g>
  </svg>`)
}

/** Solid style returns hex colors (shows initial letter). Others return SVG data URLs. */
export function generateAvatar(style: AvatarStyleId, seed: string): string {
  const rand = mulberry32(hashSeed(`${style}:${seed}`))
  switch (style) {
    case 'solid':
      return pick(rand, AVATAR_COLORS)
    case 'gradient':
      return gradientSvg(pick(rand, AVATAR_COLORS), pick(rand, AVATAR_COLORS), Math.floor(rand() * 360))
    case 'geo':
      return geoSvg(rand)
    case 'mosaic':
      return mosaicSvg(rand)
    case 'pixel':
      return pixelSvg(rand)
    case 'rings':
      return ringsSvg(rand)
    case 'wave':
      return waveSvg(rand)
    case 'blob':
      return blobSvg(rand)
    default:
      return AVATAR_COLORS[0]!
  }
}

export function randomAvatar(style?: AvatarStyleId): string {
  const s = style || pick(Math.random, AVATAR_STYLES.map((x) => x.id))
  const seed = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
  return generateAvatar(s, seed)
}

/** Build a selectable gallery for one style. */
export function avatarBatch(style: AvatarStyleId, count = 12, batchKey = ''): string[] {
  const key = batchKey || Math.random().toString(36).slice(2, 8)
  const out: string[] = []
  for (let i = 0; i < count; i++) {
    out.push(generateAvatar(style, `${key}-${style}-${i}`))
  }
  return out
}

/** Mix of styles for a quick “surprise me” strip. */
export function mixedAvatarBatch(count = 12, batchKey = ''): string[] {
  const key = batchKey || Math.random().toString(36).slice(2, 8)
  const styles = AVATAR_STYLES.map((s) => s.id)
  const out: string[] = []
  for (let i = 0; i < count; i++) {
    out.push(generateAvatar(styles[i % styles.length]!, `${key}-mix-${i}`))
  }
  return out
}
