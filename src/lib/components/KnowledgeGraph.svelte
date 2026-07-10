<script lang="ts">
  import { onMount } from 'svelte'
  import { fade } from 'svelte/transition'
  import { api, describeError, type GraphNode, type KnowledgeGraph as GraphData } from '../api'
  import { colorForIndex } from '../colors'

  let graph = $state<GraphData>({ nodes: [], edges: [] })
  let loading = $state(true)
  let error = $state<string | null>(null)
  let selected = $state<string | null>(null)
  let hoveredNode = $state<string | null>(null)
  let positions = $state<Map<string, { x: number; y: number }>>(new Map())
  let linking = $state(false)
  let unlinkingKey = $state<string | null>(null)

  const width = 640
  const height = 420
  const defaultViewBox = { x: 0, y: 0, w: width, h: height }
  let viewBox = $state({ ...defaultViewBox })
  let svgEl = $state<SVGSVGElement | undefined>(undefined)

  type DragState =
    | { type: 'node'; id: string; offsetX: number; offsetY: number; moved: boolean }
    | { type: 'pan'; startClientX: number; startClientY: number; startViewX: number; startViewY: number }
  let dragState: DragState | null = null

  function clamp(v: number, min: number, max: number) {
    return Math.max(min, Math.min(max, v))
  }

  function layout(nodes: GraphNode[], edges: { source: string; target: string }[]) {
    const n = nodes.length
    const pos = nodes.map((_, i) => {
      const angle = (i / Math.max(n, 1)) * Math.PI * 2
      return {
        x: width / 2 + Math.cos(angle) * Math.min(width, height) * 0.32,
        y: height / 2 + Math.sin(angle) * Math.min(width, height) * 0.32,
      }
    })
    const idIndex = new Map(nodes.map((node, i) => [node.id, i]))
    const vx = new Array(n).fill(0)
    const vy = new Array(n).fill(0)

    for (let iter = 0; iter < 250 && n > 1; iter++) {
      for (let i = 0; i < n; i++) {
        for (let j = i + 1; j < n; j++) {
          const dx = pos[i].x - pos[j].x
          const dy = pos[i].y - pos[j].y
          const distSq = Math.max(dx * dx + dy * dy, 1)
          const force = 1800 / distSq
          const dist = Math.sqrt(distSq)
          const fx = (dx / dist) * force
          const fy = (dy / dist) * force
          vx[i] += fx
          vy[i] += fy
          vx[j] -= fx
          vy[j] -= fy
        }
      }
      for (const edge of edges) {
        const i = idIndex.get(edge.source)
        const j = idIndex.get(edge.target)
        if (i === undefined || j === undefined) continue
        const dx = pos[j].x - pos[i].x
        const dy = pos[j].y - pos[i].y
        const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1)
        const force = (dist - 110) * 0.02
        const fx = (dx / dist) * force
        const fy = (dy / dist) * force
        vx[i] += fx
        vy[i] += fy
        vx[j] -= fx
        vy[j] -= fy
      }
      for (let i = 0; i < n; i++) {
        vx[i] += (width / 2 - pos[i].x) * 0.002
        vy[i] += (height / 2 - pos[i].y) * 0.002
        pos[i].x += vx[i] * 0.04
        pos[i].y += vy[i] * 0.04
        vx[i] *= 0.82
        vy[i] *= 0.82
        pos[i].x = Math.max(24, Math.min(width - 24, pos[i].x))
        pos[i].y = Math.max(24, Math.min(height - 24, pos[i].y))
      }
    }

    const result = new Map<string, { x: number; y: number }>()
    nodes.forEach((node, i) => result.set(node.id, pos[i]))
    return result
  }

  async function load() {
    loading = true
    error = null
    try {
      graph = await api.getKnowledgeGraph()
      positions = layout(graph.nodes, graph.edges)
      viewBox = { ...defaultViewBox }
    } catch (e) {
      error = describeError(e)
    } finally {
      loading = false
    }
  }

  onMount(load)

  const tracks = $derived([...new Set(graph.nodes.map((n) => n.track))].sort())
  const activeNode = $derived(hoveredNode ?? selected)
  const connectedEdgeKeys = $derived(
    activeNode
      ? new Set(
          graph.edges
            .filter((e) => e.source === activeNode || e.target === activeNode)
            .map((e) => e.source + e.target),
        )
      : new Set<string>(),
  )

  function trackColor(track: string) {
    return colorForIndex(tracks.indexOf(track))
  }

  function toSvgPoint(clientX: number, clientY: number) {
    if (!svgEl) return { x: 0, y: 0 }
    const rect = svgEl.getBoundingClientRect()
    return {
      x: viewBox.x + ((clientX - rect.left) / rect.width) * viewBox.w,
      y: viewBox.y + ((clientY - rect.top) / rect.height) * viewBox.h,
    }
  }

  async function onNodeClick(id: string) {
    if (linking) return
    if (!selected) {
      selected = id
      return
    }
    if (selected === id) {
      selected = null
      return
    }
    const a = selected
    selected = null
    linking = true
    try {
      await api.linkMemories(a, id)
      await load()
    } catch (e) {
      error = describeError(e)
    } finally {
      linking = false
    }
  }

  function onNodePointerDown(e: PointerEvent, id: string) {
    if (linking || e.button !== 0) return
    e.stopPropagation()
    const pos = positions.get(id)
    if (!pos) return
    const p = toSvgPoint(e.clientX, e.clientY)
    dragState = { type: 'node', id, offsetX: p.x - pos.x, offsetY: p.y - pos.y, moved: false }
  }

  function onBackgroundPointerDown(e: PointerEvent) {
    if (e.button !== 0) return
    dragState = { type: 'pan', startClientX: e.clientX, startClientY: e.clientY, startViewX: viewBox.x, startViewY: viewBox.y }
  }

  function onWindowPointerMove(e: PointerEvent) {
    if (!dragState) return
    if (dragState.type === 'node') {
      const pos = positions.get(dragState.id)
      if (!pos) return
      const p = toSvgPoint(e.clientX, e.clientY)
      const nx = clamp(p.x - dragState.offsetX, 16, width - 16)
      const ny = clamp(p.y - dragState.offsetY, 16, height - 16)
      if (Math.abs(nx - pos.x) > 3 || Math.abs(ny - pos.y) > 3) dragState.moved = true
      const next = new Map(positions)
      next.set(dragState.id, { x: nx, y: ny })
      positions = next
    } else {
      if (!svgEl) return
      const rect = svgEl.getBoundingClientRect()
      const dx = ((dragState.startClientX - e.clientX) / rect.width) * viewBox.w
      const dy = ((dragState.startClientY - e.clientY) / rect.height) * viewBox.h
      viewBox = { ...viewBox, x: dragState.startViewX + dx, y: dragState.startViewY + dy }
    }
  }

  function onWindowPointerUp() {
    if (dragState?.type === 'node' && !dragState.moved) {
      onNodeClick(dragState.id)
    }
    dragState = null
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault()
    const factor = e.deltaY > 0 ? 1.1 : 0.9
    const p = toSvgPoint(e.clientX, e.clientY)
    const newW = clamp(viewBox.w * factor, width * 0.3, width * 2.5)
    const newH = clamp(viewBox.h * factor, height * 0.3, height * 2.5)
    viewBox = {
      x: p.x - (p.x - viewBox.x) * (newW / viewBox.w),
      y: p.y - (p.y - viewBox.y) * (newH / viewBox.h),
      w: newW,
      h: newH,
    }
  }

  function resetView() {
    viewBox = { ...defaultViewBox }
  }

  async function unlink(source: string, target: string) {
    const key = source + target
    if (unlinkingKey) return
    unlinkingKey = key
    try {
      await api.unlinkMemories(source, target)
      await load()
    } catch (e) {
      error = describeError(e)
    } finally {
      unlinkingKey = null
    }
  }

  function labelFor(id: string) {
    return graph.nodes.find((n) => n.id === id)?.label ?? id
  }
</script>

<svelte:window onpointermove={onWindowPointerMove} onpointerup={onWindowPointerUp} />

<div class="flex flex-col gap-6 p-8 max-w-4xl mx-auto w-full">
  <div class="flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-medium text-[var(--text)] m-0">Knowledge graph</h1>
      <p class="text-sm text-[var(--text-muted)] mt-1">
        Click a memory, then another to link them. Drag to rearrange, scroll to zoom.
        {#if selected}Pick a second memory, or click it again to cancel.{/if}
      </p>
    </div>
    {#if graph.nodes.length > 0}
      <button
        class="shrink-0 text-xs px-2.5 py-1.5 rounded-lg border border-[var(--border)] text-[var(--text-muted)] hover:text-[var(--text)] hover:border-[var(--accent)] transition-colors"
        onclick={resetView}
      >
        Reset view
      </button>
    {/if}
  </div>

  {#if error}
    <p class="text-[var(--danger)]">{error}</p>
  {:else if loading}
    <p class="text-[var(--text-muted)]">Loading…</p>
  {:else if graph.nodes.length === 0}
    <p class="text-[var(--text-muted)]">Add a few memories first, then come back to link related ones.</p>
  {:else}
    <div
      class="rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] overflow-hidden"
      transition:fade={{ duration: 150 }}
    >
      <svg
        bind:this={svgEl}
        viewBox={`${viewBox.x} ${viewBox.y} ${viewBox.w} ${viewBox.h}`}
        class="w-full h-auto cursor-grab active:cursor-grabbing"
        role="img"
        aria-label="Knowledge graph"
        onpointerdown={onBackgroundPointerDown}
        onwheel={onWheel}
      >
        {#each graph.edges as edge (edge.source + edge.target)}
          {@const key = edge.source + edge.target}
          {@const isActive = connectedEdgeKeys.has(key)}
          {#if positions.get(edge.source) && positions.get(edge.target)}
            <line
              x1={positions.get(edge.source)?.x}
              y1={positions.get(edge.source)?.y}
              x2={positions.get(edge.target)?.x}
              y2={positions.get(edge.target)?.y}
              stroke={isActive ? 'var(--accent)' : 'var(--border)'}
              stroke-width={isActive ? 2.5 : 1.5}
              opacity={activeNode && !isActive ? 0.35 : 1}
            />
          {/if}
        {/each}
        {#each graph.nodes as node (node.id)}
          {#if positions.get(node.id)}
            <g
              transform={`translate(${positions.get(node.id)?.x}, ${positions.get(node.id)?.y})`}
              class={`cursor-pointer ${linking ? 'pointer-events-none opacity-60' : ''}`}
              role="button"
              tabindex="0"
              aria-label={`${node.label} (${node.track})${selected === node.id ? ', selected' : ''}`}
              aria-pressed={selected === node.id}
              onpointerdown={(e) => onNodePointerDown(e, node.id)}
              onpointerenter={() => (hoveredNode = node.id)}
              onpointerleave={() => (hoveredNode = null)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  onNodeClick(node.id)
                }
              }}
            >
              <circle r={selected === node.id ? 10 : 7} fill={trackColor(node.track)} stroke="var(--bg-elevated)" stroke-width="2" />
              {#if selected === node.id}
                <circle r="14" fill="none" stroke="var(--accent)" stroke-width="2" stroke-dasharray="3,3" />
              {/if}
              <text y="22" text-anchor="middle" font-size="9" class="fill-[var(--text-muted)]">
                {node.label.length > 16 ? node.label.slice(0, 16) + '…' : node.label}
              </text>
            </g>
          {/if}
        {/each}
      </svg>
    </div>

    {#if tracks.length > 0}
      <div class="flex flex-wrap gap-3">
        {#each tracks as track, i (track)}
          <div class="flex items-center gap-1.5 text-xs text-[var(--text-muted)]">
            <span class="w-2.5 h-2.5 rounded-full inline-block" style={`background: ${colorForIndex(i)}`}></span>
            {track}
          </div>
        {/each}
      </div>
    {/if}

    {#if graph.edges.length > 0}
      <div>
        <h2 class="text-lg font-medium text-[var(--text)] mb-3">Links</h2>
        <div class="flex flex-col gap-2">
          {#each graph.edges as edge (edge.source + edge.target)}
            <div class="flex items-center justify-between rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-2 text-sm">
              <span class="text-[var(--text)]">{labelFor(edge.source)} &harr; {labelFor(edge.target)}</span>
              <button
                class="text-[var(--text-muted)] hover:text-[var(--danger)] transition-colors disabled:opacity-40"
                onclick={() => unlink(edge.source, edge.target)}
                disabled={unlinkingKey !== null}
              >
                {unlinkingKey === edge.source + edge.target ? 'Unlinking…' : 'Unlink'}
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>
