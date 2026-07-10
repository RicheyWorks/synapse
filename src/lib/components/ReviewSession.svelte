<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { fade, fly } from 'svelte/transition'
  import { api, describeError, type MemoryItem } from '../api'
  import CardView from './CardView.svelte'

  let { onFinish }: { onFinish: () => void } = $props()

  // Must match synapse-core's domain::LEECH_THRESHOLD.
  const LEECH_THRESHOLD = 8
  // How long the post-score "next review in N days" pill stays up before
  // auto-advancing. Any keypress skips it immediately.
  const FEEDBACK_MS = 900

  let queue = $state<MemoryItem[]>([])
  let index = $state(0)
  let revealed = $state(false)
  let loading = $state(true)
  let error = $state<string | null>(null)
  let submitting = $state(false)
  let feedback = $state<MemoryItem | null>(null)
  let sessionScores = $state<number[]>([])
  let advanceTimer: ReturnType<typeof setTimeout> | undefined

  const current = $derived(queue[index] ?? null)
  const done = $derived(!loading && queue.length > 0 && index >= queue.length)
  const progressLabel = $derived(
    queue.length > 0 ? `${Math.min(index + 1, queue.length)} / ${queue.length}` : '',
  )
  const isLeech = $derived(current !== null && current.total_lapses >= LEECH_THRESHOLD)
  const sessionRetention = $derived(
    sessionScores.length > 0
      ? Math.round((sessionScores.filter((s) => s >= 3).length / sessionScores.length) * 100)
      : 0,
  )

  const qualityButtons = [
    { score: 0, label: 'Blackout', hint: '0' },
    { score: 1, label: 'Wrong', hint: '1' },
    { score: 2, label: 'Wrong, familiar', hint: '2' },
    { score: 3, label: 'Hard', hint: '3' },
    { score: 4, label: 'Good', hint: '4' },
    { score: 5, label: 'Easy', hint: '5' },
  ]

  async function load() {
    loading = true
    error = null
    try {
      queue = await api.startReviewSession()
    } catch (e) {
      error = describeError(e)
    } finally {
      loading = false
    }
  }

  onMount(load)
  onDestroy(() => clearTimeout(advanceTimer))

  function reveal() {
    if (!current) return
    revealed = true
  }

  function formatInterval(days: number): string {
    if (days <= 0) return 'again soon'
    if (days === 1) return 'tomorrow'
    return `in ${days} days`
  }

  function advance() {
    clearTimeout(advanceTimer)
    feedback = null
    index += 1
    revealed = false
  }

  async function score(value: number) {
    if (!current || submitting || feedback) return
    submitting = true
    try {
      const updated = await api.reviewMemory(current.id, value)
      sessionScores = [...sessionScores, value]
      feedback = updated
      advanceTimer = setTimeout(advance, FEEDBACK_MS)
    } catch (e) {
      error = describeError(e)
    } finally {
      submitting = false
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (done || loading) return
    if (e.key === 'Escape') {
      onFinish()
      return
    }
    if (feedback) {
      // Any key skips the brief post-score pill and moves on immediately.
      e.preventDefault()
      advance()
      return
    }
    if (!revealed) {
      if (e.key === ' ' || e.key === 'Enter') {
        e.preventDefault()
        reveal()
      }
      return
    }
    if (e.key >= '0' && e.key <= '5') {
      score(Number(e.key))
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex flex-col h-full max-w-2xl mx-auto w-full p-8">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-xl font-medium text-[var(--text)] m-0">Review</h1>
    <div class="flex items-center gap-4">
      {#if progressLabel}
        <span class="text-sm text-[var(--text-muted)]">{progressLabel}</span>
      {/if}
      <button
        class="text-sm text-[var(--text-muted)] hover:text-[var(--text)] transition-colors"
        onclick={onFinish}
      >
        End (Esc)
      </button>
    </div>
  </div>

  {#if error}
    <p class="text-[var(--danger)]">{error}</p>
  {:else if loading}
    <p class="text-[var(--text-muted)]">Loading session…</p>
  {:else if queue.length === 0}
    <div class="flex flex-col items-center justify-center flex-1 gap-3 text-center">
      <p class="text-lg text-[var(--text)]">Nothing due right now.</p>
      <p class="text-[var(--text-muted)]">Come back later, or add more memories.</p>
      <button
        class="mt-2 rounded-lg px-4 py-2 font-medium bg-[var(--accent)] text-[var(--accent-contrast)] hover:bg-[var(--accent-hover)] transition-colors"
        onclick={onFinish}
      >
        Back to dashboard
      </button>
    </div>
  {:else if done}
    <div class="flex flex-col items-center justify-center flex-1 gap-3 text-center">
      <p class="text-lg text-[var(--text)]">Session complete.</p>
      <p class="text-[var(--text-muted)]">
        Reviewed {queue.length} item{queue.length === 1 ? '' : 's'} — {sessionRetention}% retention.
      </p>
      <button
        class="mt-2 rounded-lg px-4 py-2 font-medium bg-[var(--accent)] text-[var(--accent-contrast)] hover:bg-[var(--accent-hover)] transition-colors"
        onclick={onFinish}
      >
        Back to dashboard
      </button>
    </div>
  {:else if current}
    <div class="flex flex-col flex-1 gap-6">
      <div
        class="relative flex-1 flex flex-col items-center justify-center gap-4 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-8 text-center"
      >
        <div class="flex items-center gap-2">
          <span class="text-xs uppercase tracking-wide text-[var(--text-muted)]">{current.training_track}</span>
          {#if isLeech}
            <span
              class="text-xs px-1.5 py-0.5 rounded-full bg-[var(--bg-inset)] text-[var(--warning)] border border-[var(--warning)]"
              title={`This memory has failed ${current.total_lapses} times — consider rewriting it.`}
            >
              leech
            </span>
          {/if}
        </div>
        <p class="text-xl text-[var(--text)]">{current.prompt}</p>
        {#if revealed || current.card.type === 'cloze'}
          <div class="w-full border-t border-[var(--border)] pt-4 mt-2" in:fade={{ duration: 150 }}>
            <CardView card={current.card} {revealed} />
          </div>
        {/if}

        {#if feedback}
          <div class="absolute inset-x-0 bottom-3 flex justify-center" in:fly={{ y: 6, duration: 150 }}>
            <span class="text-xs px-2.5 py-1 rounded-full bg-[var(--bg-inset)] text-[var(--text-muted)]">
              Next review {formatInterval(feedback.interval_days)}
            </span>
          </div>
        {/if}
      </div>

      {#if !revealed && !feedback}
        <button
          class="rounded-lg px-4 py-3 font-medium bg-[var(--accent)] text-[var(--accent-contrast)] hover:bg-[var(--accent-hover)] transition-colors"
          onclick={reveal}
        >
          Reveal (Space)
        </button>
      {:else if !feedback}
        <div class="grid grid-cols-3 sm:grid-cols-6 gap-2" in:fly={{ y: 8, duration: 150 }}>
          {#each qualityButtons as btn (btn.score)}
            <button
              class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-2 py-3 text-sm text-[var(--text)] hover:border-[var(--accent)] transition-colors disabled:opacity-40"
              disabled={submitting}
              onclick={() => score(btn.score)}
            >
              <div class="font-semibold">{btn.hint}</div>
              <div class="text-xs text-[var(--text-muted)]">{btn.label}</div>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
