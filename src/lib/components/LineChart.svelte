<script lang="ts">
  interface Point {
    x: number
    y: number
    label: string
  }

  let {
    points,
    yDomain,
    yFormat = (y: number) => String(y),
    xFormat = (x: number) => String(x),
    height = 180,
  }: {
    points: Point[]
    yDomain?: [number, number]
    yFormat?: (y: number) => string
    xFormat?: (x: number) => string
    height?: number
  } = $props()

  const width = 600
  const padding = { top: 12, right: 12, bottom: 24, left: 12 }
  const innerW = width - padding.left - padding.right
  const innerH = $derived(height - padding.top - padding.bottom)

  const xMin = $derived(points.length ? Math.min(...points.map((p) => p.x)) : 0)
  const xMax = $derived(points.length ? Math.max(...points.map((p) => p.x)) : 1)
  const yBounds = $derived.by((): [number, number] => {
    if (yDomain) return yDomain
    if (!points.length) return [0, 1]
    return [Math.min(...points.map((p) => p.y)), Math.max(...points.map((p) => p.y))]
  })

  function sx(x: number) {
    return padding.left + ((x - xMin) / Math.max(xMax - xMin, 1e-6)) * innerW
  }
  function sy(y: number) {
    const [yMin, yMax] = yBounds
    return padding.top + innerH - ((y - yMin) / Math.max(yMax - yMin, 1e-6)) * innerH
  }

  const linePath = $derived(
    points.length ? points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${sx(p.x)} ${sy(p.y)}`).join(' ') : '',
  )
  const areaPath = $derived(
    points.length
      ? `${linePath} L ${sx(points[points.length - 1].x)} ${padding.top + innerH} L ${sx(points[0].x)} ${padding.top + innerH} Z`
      : '',
  )

  let hoverIndex = $state<number | null>(null)

  function onMove(e: MouseEvent) {
    if (points.length === 0) return
    const svg = e.currentTarget as SVGSVGElement
    const rect = svg.getBoundingClientRect()
    const mx = ((e.clientX - rect.left) / rect.width) * width
    let closest = 0
    let closestDist = Infinity
    points.forEach((p, i) => {
      const d = Math.abs(sx(p.x) - mx)
      if (d < closestDist) {
        closestDist = d
        closest = i
      }
    })
    hoverIndex = closest
  }

  const hovered = $derived(hoverIndex !== null ? points[hoverIndex] : null)
</script>

{#if points.length === 0}
  <p class="text-[var(--text-muted)] text-sm">Not enough data yet.</p>
{:else}
  <div class="relative w-full">
    <svg
      viewBox={`0 0 ${width} ${height}`}
      class="w-full h-auto"
      onmousemove={onMove}
      onmouseleave={() => (hoverIndex = null)}
      role="img"
      aria-label="Line chart"
    >
      <line
        x1={padding.left}
        y1={padding.top + innerH}
        x2={width - padding.right}
        y2={padding.top + innerH}
        stroke="var(--border)"
        stroke-width="1"
      />

      <path d={areaPath} fill="var(--accent)" opacity="0.12" stroke="none" />
      <path
        d={linePath}
        fill="none"
        stroke="var(--accent)"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      />

      {#if points.length === 1}
        <circle cx={sx(points[0].x)} cy={sy(points[0].y)} r="3" fill="var(--accent)" />
      {/if}

      {#if hovered}
        <line
          x1={sx(hovered.x)}
          y1={padding.top}
          x2={sx(hovered.x)}
          y2={padding.top + innerH}
          stroke="var(--text-muted)"
          stroke-width="1"
          stroke-dasharray="3,3"
        />
        <circle cx={sx(hovered.x)} cy={sy(hovered.y)} r="4" fill="var(--accent)" stroke="var(--bg-elevated)" stroke-width="2" />
      {/if}

      <text x={padding.left} y={height - 6} font-size="10" class="fill-[var(--text-muted)]">{xFormat(xMin)}</text>
      <text x={width - padding.right} y={height - 6} text-anchor="end" font-size="10" class="fill-[var(--text-muted)]">
        {xFormat(xMax)}
      </text>
    </svg>
    {#if hovered}
      <div
        class="pointer-events-none absolute rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-2 py-1 text-xs text-[var(--text)] shadow-lg whitespace-nowrap"
        style={`left: ${(sx(hovered.x) / width) * 100}%; top: 0; transform: translate(-50%, -110%);`}
      >
        {xFormat(hovered.x)}: {yFormat(hovered.y)}
      </div>
    {/if}
  </div>
{/if}
