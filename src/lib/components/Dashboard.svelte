<script lang="ts">
  import { onMount } from 'svelte'
  import { api, describeError, type Stats, type TrackSummary } from '../api'

  let { onStartReview }: { onStartReview: () => void } = $props()

  let stats = $state<Stats | null>(null)
  let tracks = $state<TrackSummary[]>([])
  let error = $state<string | null>(null)
  let loading = $state(true)

  async function load() {
    loading = true
    error = null
    try {
      const [s, t] = await Promise.all([api.getStats(), api.listTracks()])
      stats = s
      tracks = t
    } catch (e) {
      error = describeError(e)
    } finally {
      loading = false
    }
  }

  onMount(load)

  const tiles = $derived(
    stats
      ? [
          { label: 'Due now', value: stats.due_now.toString() },
          { label: 'Current streak', value: `${stats.current_streak_days}d` },
          { label: 'Best streak', value: `${stats.best_streak_days}d` },
          { label: 'Retention', value: `${Math.round(stats.retention_rate * 100)}%` },
          { label: 'Avg. ease', value: stats.average_ease.toFixed(2) },
          { label: 'Leeches', value: stats.leech_count.toString() },
        ]
      : [],
  )
</script>

<div class="flex flex-col gap-8 p-8 max-w-4xl mx-auto w-full">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-medium text-[var(--text)] m-0">Dashboard</h1>
    <button
      class="rounded-lg px-4 py-2 font-medium bg-[var(--accent)] text-[var(--accent-contrast)] hover:bg-[var(--accent-hover)] transition-colors disabled:opacity-40"
      disabled={loading || !stats || stats.due_now === 0}
      onclick={onStartReview}
    >
      Start review{stats && stats.due_now > 0 ? ` (${stats.due_now})` : ''}
    </button>
  </div>

  {#if error}
    <p class="text-[var(--danger)]">{error}</p>
  {:else if loading}
    <p class="text-[var(--text-muted)]">Loading…</p>
  {:else}
    <div class="grid grid-cols-2 sm:grid-cols-3 gap-4">
      {#each tiles as tile (tile.label)}
        <div class="rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-4">
          <div class="text-xs uppercase tracking-wide text-[var(--text-muted)]">{tile.label}</div>
          <div class="text-2xl font-semibold text-[var(--text)] mt-1">{tile.value}</div>
        </div>
      {/each}
    </div>

    <div>
      <h2 class="text-lg font-medium text-[var(--text)] mb-3">Tracks</h2>
      {#if tracks.length === 0}
        <p class="text-[var(--text-muted)]">No tracks yet — add a memory to get started.</p>
      {:else}
        <div class="flex flex-col gap-2">
          {#each tracks as track (track.name)}
            <div
              class="flex items-center justify-between rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-3"
            >
              <span class="text-[var(--text)]">{track.name}</span>
              <span class="text-sm text-[var(--text-muted)]">
                {track.total} item{track.total === 1 ? '' : 's'}
                {#if track.due > 0}
                  <span class="ml-2 rounded-full bg-[var(--accent)] text-[var(--accent-contrast)] px-2 py-0.5 text-xs font-medium">
                    {track.due} due
                  </span>
                {/if}
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
