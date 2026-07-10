<script lang="ts">
  import { save, open } from '@tauri-apps/api/dialog'
  import { api, describeError, type Settings, type ThemeId } from '../api'

  let {
    settings,
    onChange,
  }: { settings: Settings; onChange: (s: Settings) => void } = $props()

  let saving = $state(false)
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)

  const themes: { id: ThemeId; label: string; description: string }[] = [
    { id: 'neural', label: 'Neural', description: 'Clean, calm, modern' },
    { id: 'blackbeard', label: 'Blackbeard', description: 'Pirate-flavored, still sharp' },
  ]

  async function persist(next: Settings) {
    saving = true
    error = null
    try {
      const applied = await api.updateSettings(next)
      onChange(applied)
    } catch (e) {
      error = describeError(e)
    } finally {
      saving = false
    }
  }

  function setTheme(id: ThemeId) {
    persist({ ...settings, theme: id })
  }

  function setLimit(e: Event) {
    const value = Number((e.target as HTMLInputElement).value)
    if (!Number.isFinite(value) || value <= 0) return
    persist({ ...settings, daily_review_limit: value })
  }

  async function doExport() {
    error = null
    notice = null
    const path = await save({
      title: 'Export memories',
      defaultPath: 'synapse-backup.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path) return
    try {
      await api.exportMemories(path)
      notice = `Exported to ${path}`
    } catch (e) {
      error = describeError(e)
    }
  }

  async function doImport() {
    error = null
    notice = null
    const path = await open({
      title: 'Import memories',
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path || Array.isArray(path)) return
    try {
      const count = await api.importMemories(path)
      notice = `Imported ${count} item${count === 1 ? '' : 's'}`
    } catch (e) {
      error = describeError(e)
    }
  }
</script>

<div class="flex flex-col gap-8 p-8 max-w-2xl mx-auto w-full">
  <h1 class="text-2xl font-medium text-[var(--text)] m-0">Settings</h1>

  <section class="flex flex-col gap-3">
    <h2 class="text-lg font-medium text-[var(--text)] m-0">Theme</h2>
    <div class="grid grid-cols-2 gap-3">
      {#each themes as theme (theme.id)}
        <button
          class={`text-left rounded-xl border p-4 transition-colors ${
            settings.theme === theme.id ? 'border-[var(--accent)]' : 'border-[var(--border)]'
          }`}
          style="background: var(--bg-elevated);"
          onclick={() => setTheme(theme.id)}
          disabled={saving}
        >
          <div class="font-medium text-[var(--text)]">{theme.label}</div>
          <div class="text-sm text-[var(--text-muted)]">{theme.description}</div>
        </button>
      {/each}
    </div>
  </section>

  <section class="flex flex-col gap-3">
    <h2 class="text-lg font-medium text-[var(--text)] m-0">Review session size</h2>
    <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)] max-w-xs">
      Daily review limit
      <input
        type="number"
        min="1"
        class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)]"
        value={settings.daily_review_limit}
        onchange={setLimit}
        disabled={saving}
      />
    </label>
  </section>

  <section class="flex flex-col gap-3">
    <h2 class="text-lg font-medium text-[var(--text)] m-0">Backup</h2>
    <div class="flex gap-3">
      <button
        class="rounded-lg px-4 py-2 font-medium border border-[var(--border)] text-[var(--text)] hover:border-[var(--accent)] transition-colors"
        onclick={doExport}
      >
        Export…
      </button>
      <button
        class="rounded-lg px-4 py-2 font-medium border border-[var(--border)] text-[var(--text)] hover:border-[var(--accent)] transition-colors"
        onclick={doImport}
      >
        Import…
      </button>
    </div>
  </section>

  {#if notice}
    <p class="text-[var(--success)] text-sm">{notice}</p>
  {/if}
  {#if error}
    <p class="text-[var(--danger)] text-sm">{error}</p>
  {/if}
</div>
