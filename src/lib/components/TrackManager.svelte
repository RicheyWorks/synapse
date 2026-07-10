<script lang="ts">
  import { onMount } from 'svelte'
  import { api, describeError, type TrackSummary } from '../api'

  let tracks = $state<TrackSummary[]>([])
  let loading = $state(true)
  let error = $state<string | null>(null)
  let saving = $state(false)
  let saved = $state(false)

  let track = $state('')
  let prompt = $state('')
  let content = $state('')

  async function load() {
    loading = true
    error = null
    try {
      tracks = await api.listTracks()
    } catch (e) {
      error = describeError(e)
    } finally {
      loading = false
    }
  }

  onMount(load)

  const canSubmit = $derived(track.trim() !== '' && prompt.trim() !== '' && content.trim() !== '')

  async function submit(e: Event) {
    e.preventDefault()
    if (!canSubmit || saving) return
    saving = true
    error = null
    saved = false
    try {
      await api.addMemory(track.trim(), prompt.trim(), content.trim())
      prompt = ''
      content = ''
      saved = true
      await load()
    } catch (e) {
      error = describeError(e)
    } finally {
      saving = false
    }
  }
</script>

<div class="flex flex-col gap-8 p-8 max-w-3xl mx-auto w-full">
  <h1 class="text-2xl font-medium text-[var(--text)] m-0">Tracks</h1>

  <form class="flex flex-col gap-3 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-6" onsubmit={submit}>
    <h2 class="text-lg font-medium text-[var(--text)] m-0">Add a memory</h2>

    <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
      Track
      <input
        class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)]"
        placeholder="e.g. Rust Programming"
        list="track-names"
        bind:value={track}
      />
      <datalist id="track-names">
        {#each tracks as t (t.name)}
          <option value={t.name}></option>
        {/each}
      </datalist>
    </label>

    <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
      Prompt
      <textarea
        class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)] min-h-16"
        placeholder="What triggers the recall?"
        bind:value={prompt}
      ></textarea>
    </label>

    <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
      Content
      <textarea
        class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)] min-h-24"
        placeholder="The answer / fact to remember"
        bind:value={content}
      ></textarea>
    </label>

    <button
      type="submit"
      class="self-start rounded-lg px-4 py-2 font-medium bg-[var(--accent)] text-[var(--accent-contrast)] hover:bg-[var(--accent-hover)] transition-colors disabled:opacity-40"
      disabled={!canSubmit || saving}
    >
      {saving ? 'Adding…' : 'Add memory'}
    </button>

    {#if saved}
      <p class="text-[var(--success)] text-sm">Added.</p>
    {/if}
    {#if error}
      <p class="text-[var(--danger)] text-sm">{error}</p>
    {/if}
  </form>

  <div>
    <h2 class="text-lg font-medium text-[var(--text)] mb-3">All tracks</h2>
    {#if loading}
      <p class="text-[var(--text-muted)]">Loading…</p>
    {:else if tracks.length === 0}
      <p class="text-[var(--text-muted)]">No tracks yet.</p>
    {:else}
      <div class="flex flex-col gap-2">
        {#each tracks as t (t.name)}
          <div class="flex items-center justify-between rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-3">
            <span class="text-[var(--text)]">{t.name}</span>
            <span class="text-sm text-[var(--text-muted)]">{t.total} total · {t.due} due</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
