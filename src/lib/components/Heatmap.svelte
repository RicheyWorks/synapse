<script lang="ts">
  import type { HeatmapDay } from '../api'

  let { days }: { days: HeatmapDay[] } = $props()

  const maxCount = $derived(Math.max(1, ...days.map((d) => d.review_count)))

  function styleFor(count: number) {
    if (count === 0) return 'background: var(--bg-inset);'
    const ratio = count / maxCount
    const opacity = 0.28 + ratio * 0.72
    return `background: var(--accent); opacity: ${opacity};`
  }

  // Pad the front so the grid's first column starts on a Sunday.
  const weeks = $derived.by(() => {
    if (days.length === 0) return [] as (HeatmapDay | null)[][]
    const first = new Date(`${days[0].date}T00:00:00Z`)
    const pad = first.getUTCDay()
    const cells: (HeatmapDay | null)[] = [...Array(pad).fill(null), ...days]
    const result: (HeatmapDay | null)[][] = []
    for (let i = 0; i < cells.length; i += 7) result.push(cells.slice(i, i + 7))
    return result
  })

  let hovered = $state<HeatmapDay | null>(null)
  let hoverPos = $state({ x: 0, y: 0 })

  function onEnter(day: HeatmapDay | null, e: MouseEvent) {
    if (!day) return
    hovered = day
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
    hoverPos = { x: rect.left + rect.width / 2, y: rect.top }
  }
</script>

{#if days.length === 0}
  <p class="text-[var(--text-muted)] text-sm">No review activity yet.</p>
{:else}
  <div class="relative">
    <div class="flex gap-1">
      {#each weeks as week, wi (wi)}
        <div class="flex flex-col gap-1">
          {#each week as day, di (di)}
            {#if day}
              <div
                role="img"
                aria-label={`${day.date}: ${day.review_count} review${day.review_count === 1 ? '' : 's'}`}
                class="w-3 h-3 rounded-[3px] cursor-default"
                style={styleFor(day.review_count)}
                onmouseenter={(e) => onEnter(day, e)}
                onmouseleave={() => (hovered = null)}
              ></div>
            {:else}
              <div class="w-3 h-3"></div>
            {/if}
          {/each}
        </div>
      {/each}
    </div>
    {#if hovered}
      <div
        class="fixed z-10 -translate-x-1/2 -translate-y-full -mt-2 rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-2 py-1 text-xs text-[var(--text)] shadow-lg pointer-events-none"
        style={`left: ${hoverPos.x}px; top: ${hoverPos.y}px;`}
      >
        {hovered.review_count} review{hovered.review_count === 1 ? '' : 's'} · {hovered.date}
      </div>
    {/if}
  </div>
{/if}
