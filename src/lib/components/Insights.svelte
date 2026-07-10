<script lang="ts">
  import { onMount } from 'svelte'
  import { fade } from 'svelte/transition'
  import {
    api,
    describeError,
    type GamificationSummary,
    type HeatmapDay,
    type MemoryItem,
    type RetentionPoint,
  } from '../api'
  import Heatmap from './Heatmap.svelte'
  import LineChart from './LineChart.svelte'

  let gamification = $state<GamificationSummary | null>(null)
  let heatmap = $state<HeatmapDay[]>([])
  let retention = $state<RetentionPoint[]>([])
  let hardest = $state<MemoryItem[]>([])
  let selectedId = $state<string | null>(null)
  let forgettingCurve = $state<[number, number][]>([])
  let loading = $state(true)
  let loadingCurve = $state(false)
  let error = $state<string | null>(null)

  async function load() {
    loading = true
    error = null
    try {
      const [g, h, r, worst] = await Promise.all([
        api.getGamification(),
        api.getReviewHeatmap(90),
        api.getRetentionCurve(),
        api.getHardestItems(5),
      ])
      gamification = g
      heatmap = h
      retention = r
      hardest = worst
    } catch (e) {
      error = describeError(e)
    } finally {
      loading = false
    }
  }

  onMount(load)

  async function selectItem(item: MemoryItem) {
    if (selectedId === item.id) {
      selectedId = null
      return
    }
    selectedId = item.id
    loadingCurve = true
    try {
      forgettingCurve = await api.getForgettingCurve(item.id, 30)
    } catch (e) {
      error = describeError(e)
    } finally {
      loadingCurve = false
    }
  }

  const xpProgress = $derived(gamification ? (gamification.xp % 500) / 500 : 0)

  const retentionPoints = $derived(
    retention.map((r) => ({ x: new Date(`${r.date}T00:00:00Z`).getTime(), y: r.retention_rate * 100, label: r.date })),
  )
  const forgettingPoints = $derived(forgettingCurve.map(([d, p]) => ({ x: d, y: p * 100, label: `day ${d}` })))
</script>

<div class="flex flex-col gap-8 p-8 max-w-4xl mx-auto w-full">
  <h1 class="text-2xl font-medium text-[var(--text)] m-0">Insights</h1>

  {#if error}
    <p class="text-[var(--danger)]">{error}</p>
  {/if}

  {#if loading}
    <p class="text-[var(--text-muted)]">Loading…</p>
  {:else}
    {#if gamification}
      <section
        class="rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-6 flex flex-col gap-3"
        transition:fade={{ duration: 150 }}
      >
        <div class="flex items-center justify-between">
          <div>
            <div class="text-xs uppercase tracking-wide text-[var(--text-muted)]">Level {gamification.level}</div>
            <div class="text-xl font-medium text-[var(--text)]" style="font-family: var(--font-heading);">
              {gamification.title}
            </div>
          </div>
          <div class="text-2xl font-semibold text-[var(--text)]">{gamification.xp} XP</div>
        </div>
        <div class="h-2 rounded-full bg-[var(--bg-inset)] overflow-hidden">
          <div class="h-full bg-[var(--accent)] transition-all" style={`width: ${xpProgress * 100}%`}></div>
        </div>
        <div class="grid grid-cols-2 sm:grid-cols-3 gap-2 mt-2">
          {#each gamification.achievements as a (a.id)}
            <div
              class="rounded-lg border px-3 py-2"
              style={a.unlocked
                ? 'border-color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, transparent);'
                : 'border-color: var(--border); opacity: 0.5;'}
              aria-label={`${a.title}: ${a.description} (${a.unlocked ? 'unlocked' : 'locked'})`}
            >
              <div class="text-sm font-medium text-[var(--text)]">
                {a.title}
                <span class="text-xs text-[var(--text-muted)] font-normal">{a.unlocked ? '' : '(locked)'}</span>
              </div>
              <div class="text-xs text-[var(--text-muted)]">{a.description}</div>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    <section>
      <h2 class="text-lg font-medium text-[var(--text)] mb-3">Review activity</h2>
      <div class="rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-4 overflow-x-auto">
        <Heatmap days={heatmap} />
      </div>
    </section>

    <section>
      <h2 class="text-lg font-medium text-[var(--text)] mb-3">Retention over time</h2>
      <div class="rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-4">
        <LineChart
          points={retentionPoints}
          yDomain={[0, 100]}
          yFormat={(y) => `${Math.round(y)}%`}
          xFormat={(x) => new Date(x).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}
        />
      </div>
    </section>

    <section>
      <h2 class="text-lg font-medium text-[var(--text)] mb-3">Hardest memories</h2>
      {#if hardest.length === 0}
        <p class="text-[var(--text-muted)]">No struggling memories yet — nice.</p>
      {:else}
        <div class="flex flex-col gap-2">
          {#each hardest as item (item.id)}
            <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)]">
              <button
                class="w-full flex items-center justify-between px-4 py-3 text-left"
                aria-expanded={selectedId === item.id}
                onclick={() => selectItem(item)}
              >
                <span class="text-[var(--text)]">{item.prompt}</span>
                <span class="text-sm text-[var(--danger)]">{item.total_lapses} lapse{item.total_lapses === 1 ? '' : 's'}</span>
              </button>
              {#if selectedId === item.id}
                <div class="px-4 pb-4 flex flex-col gap-4" transition:fade={{ duration: 120 }}>
                  <div>
                    <h3 class="text-sm font-medium text-[var(--text-muted)] mb-2">Projected recall</h3>
                    {#if loadingCurve}
                      <p class="text-sm text-[var(--text-muted)]">Loading…</p>
                    {:else}
                      <LineChart
                        points={forgettingPoints}
                        yDomain={[0, 100]}
                        yFormat={(y) => `${Math.round(y)}%`}
                        xFormat={(x) => `d${x}`}
                      />
                    {/if}
                  </div>
                  <div>
                    <h3 class="text-sm font-medium text-[var(--text-muted)] mb-2">Review history</h3>
                    {#if item.review_log.length === 0}
                      <p class="text-sm text-[var(--text-muted)]">Not reviewed yet.</p>
                    {:else}
                      <div class="flex flex-col gap-1">
                        {#each [...item.review_log].reverse() as entry, i (i)}
                          <div class="flex items-center justify-between text-sm border-b border-[var(--border)] py-1.5 last:border-0">
                            <span class="text-[var(--text-muted)]">{new Date(entry.reviewed_at).toLocaleDateString()}</span>
                            <span class="text-[var(--text)]">score {entry.score}</span>
                            <span class="text-[var(--text-muted)]">{entry.interval_before_days}d &rarr; {entry.interval_after_days}d</span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>
