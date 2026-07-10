<script lang="ts">
  import { onMount } from 'svelte'
  import { api, describeError, type CardContent, type TrackSummary } from '../api'

  let tracks = $state<TrackSummary[]>([])
  let loading = $state(true)
  let error = $state<string | null>(null)
  let saving = $state(false)
  let saved = $state(false)

  let track = $state('')
  let prompt = $state('')
  let cardType = $state<CardContent['type']>('basic')

  let answer = $state('')
  let clozeText = $state('')
  let language = $state('')
  let code = $state('')
  let imagePath = $state('')
  let imageCaption = $state('')

  const cardTypes: { id: CardContent['type']; label: string }[] = [
    { id: 'basic', label: 'Basic' },
    { id: 'cloze', label: 'Cloze' },
    { id: 'code', label: 'Code' },
    { id: 'image', label: 'Image' },
  ]

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

  function buildCard(): CardContent | null {
    switch (cardType) {
      case 'basic':
        return answer.trim() ? { type: 'basic', answer: answer.trim() } : null
      case 'cloze':
        return clozeText.trim() ? { type: 'cloze', text: clozeText.trim() } : null
      case 'code':
        return code.trim() ? { type: 'code', language: language.trim() || 'text', code: code.trim() } : null
      case 'image':
        return imagePath.trim() ? { type: 'image', path: imagePath.trim(), caption: imageCaption.trim() || null } : null
    }
  }

  const canSubmit = $derived(track.trim() !== '' && prompt.trim() !== '' && buildCard() !== null)

  function resetCardFields() {
    answer = ''
    clozeText = ''
    language = ''
    code = ''
    imagePath = ''
    imageCaption = ''
  }

  async function submit(e: Event) {
    e.preventDefault()
    const card = buildCard()
    if (!canSubmit || saving || !card) return
    saving = true
    error = null
    saved = false
    try {
      await api.addMemory(track.trim(), prompt.trim(), card)
      prompt = ''
      resetCardFields()
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

    <div class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
      Card type
      <div class="flex gap-1">
        {#each cardTypes as ct (ct.id)}
          <button
            type="button"
            class={`rounded-lg px-3 py-1.5 text-sm transition-colors ${
              cardType === ct.id
                ? 'bg-[var(--accent)] text-[var(--accent-contrast)]'
                : 'bg-[var(--bg-inset)] text-[var(--text-muted)] hover:text-[var(--text)]'
            }`}
            onclick={() => (cardType = ct.id)}
          >
            {ct.label}
          </button>
        {/each}
      </div>
    </div>

    {#if cardType === 'basic'}
      <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
        Answer
        <textarea
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)] min-h-24"
          placeholder="The answer / fact to remember"
          bind:value={answer}
        ></textarea>
      </label>
    {:else if cardType === 'cloze'}
      <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
        Cloze text
        <textarea
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)] min-h-24"
          placeholder={'The {{c1::mitochondria}} is the powerhouse of the cell'}
          bind:value={clozeText}
        ></textarea>
        <span class="text-xs text-[var(--text-muted)]">
          Wrap the hidden part in <code class="text-[var(--text)]">{'{{c1::...}}'}</code>
        </span>
      </label>
    {:else if cardType === 'code'}
      <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
        Language
        <input
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)]"
          placeholder="rust"
          bind:value={language}
        />
      </label>
      <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
        Code
        <textarea
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)] min-h-24 font-mono"
          placeholder={'fn main() {}'}
          bind:value={code}
        ></textarea>
      </label>
    {:else if cardType === 'image'}
      <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
        Image path or URL
        <input
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)]"
          placeholder="/path/to/diagram.png"
          bind:value={imagePath}
        />
      </label>
      <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)]">
        Caption (optional)
        <input
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)]"
          bind:value={imageCaption}
        />
      </label>
    {/if}

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
