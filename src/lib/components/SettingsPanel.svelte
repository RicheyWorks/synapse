<script lang="ts">
  import { onMount } from 'svelte'
  import { fade } from 'svelte/transition'
  import { save, open } from '@tauri-apps/api/dialog'
  import { api, describeError, type BackupInfo, type SchedulerId, type Settings, type ThemeId } from '../api'
  import ConfirmDialog from './ConfirmDialog.svelte'

  let {
    settings,
    onChange,
  }: { settings: Settings; onChange: (s: Settings) => void } = $props()

  let saving = $state(false)
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)

  let exporting = $state(false)
  let importing = $state(false)

  let backups = $state<BackupInfo[]>([])
  let loadingBackups = $state(true)
  let backingUp = $state(false)
  let restoringFilename = $state<string | null>(null)
  let confirmRestoreFilename = $state<string | null>(null)
  let restoreTrigger = $state<HTMLElement | null>(null)

  const themes: { id: ThemeId; label: string; description: string }[] = [
    { id: 'neural', label: 'Neural', description: 'Clean, calm, modern' },
    { id: 'blackbeard', label: 'Blackbeard', description: 'Pirate-flavored, still sharp' },
  ]

  const schedulers: { id: SchedulerId; label: string; description: string }[] = [
    { id: 'sm2', label: 'SM-2', description: 'Classic, predictable, ease-factor based' },
    { id: 'fsrs', label: 'FSRS', description: 'Modern, models per-memory difficulty and stability' },
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

  function setScheduler(id: SchedulerId) {
    persist({ ...settings, scheduler: id })
  }

  function setDesiredRetention(e: Event) {
    const percent = Number((e.target as HTMLInputElement).value)
    if (!Number.isFinite(percent)) return
    persist({ ...settings, fsrs_desired_retention: percent / 100 })
  }

  async function doExport() {
    const path = await save({
      title: 'Export memories',
      defaultPath: 'synapse-backup.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path) return
    error = null
    notice = null
    exporting = true
    try {
      await api.exportMemories(path)
      notice = `Exported to ${path}`
    } catch (e) {
      error = describeError(e)
    } finally {
      exporting = false
    }
  }

  async function doImport() {
    const path = await open({
      title: 'Import memories',
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path || Array.isArray(path)) return
    error = null
    notice = null
    importing = true
    try {
      const count = await api.importMemories(path)
      notice = `Imported ${count} item${count === 1 ? '' : 's'}`
      await loadBackups() // import takes its own safety snapshot
    } catch (e) {
      error = describeError(e)
    } finally {
      importing = false
    }
  }

  async function loadBackups() {
    loadingBackups = true
    try {
      backups = await api.listBackups()
    } catch (e) {
      error = describeError(e)
    } finally {
      loadingBackups = false
    }
  }

  onMount(loadBackups)

  async function doCreateBackup() {
    backingUp = true
    error = null
    notice = null
    try {
      const filename = await api.createManualBackup()
      notice = `Backup created: ${filename}`
      await loadBackups()
    } catch (e) {
      error = describeError(e)
    } finally {
      backingUp = false
    }
  }

  function requestRestore(filename: string, e: MouseEvent) {
    restoreTrigger = e.currentTarget as HTMLElement
    confirmRestoreFilename = filename
  }

  function cancelRestore() {
    confirmRestoreFilename = null
    restoreTrigger?.focus()
    restoreTrigger = null
  }

  async function confirmRestore() {
    const filename = confirmRestoreFilename
    if (!filename) return
    confirmRestoreFilename = null
    restoringFilename = filename
    error = null
    notice = null
    try {
      const count = await api.restoreBackup(filename)
      notice = `Restored ${count} item${count === 1 ? '' : 's'} from ${filename}`
      await loadBackups()
    } catch (e) {
      error = describeError(e)
    } finally {
      restoringFilename = null
      restoreTrigger?.focus()
      restoreTrigger = null
    }
  }

  function formatBackupDate(iso: string) {
    return new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
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
          aria-pressed={settings.theme === theme.id}
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
        class="rounded-lg border border-[var(--border)] bg-[var(--bg-inset)] px-3 py-2 text-[var(--text)] outline-none focus:border-[var(--accent)] focus:ring-2 focus:ring-[var(--accent)]"
        value={settings.daily_review_limit}
        onchange={setLimit}
        disabled={saving}
      />
    </label>
  </section>

  <section class="flex flex-col gap-3">
    <h2 class="text-lg font-medium text-[var(--text)] m-0">Scheduling algorithm</h2>
    <p class="text-sm text-[var(--text-muted)] m-0">
      Switching takes effect the next time each memory is reviewed — existing items keep whatever
      their prior scheduler already set, there's no bulk migration.
    </p>
    <div class="grid grid-cols-2 gap-3">
      {#each schedulers as sched (sched.id)}
        <button
          class={`text-left rounded-xl border p-4 transition-colors ${
            settings.scheduler === sched.id ? 'border-[var(--accent)]' : 'border-[var(--border)]'
          }`}
          style="background: var(--bg-elevated);"
          aria-pressed={settings.scheduler === sched.id}
          onclick={() => setScheduler(sched.id)}
          disabled={saving}
        >
          <div class="font-medium text-[var(--text)]">{sched.label}</div>
          <div class="text-sm text-[var(--text-muted)]">{sched.description}</div>
        </button>
      {/each}
    </div>

    {#if settings.scheduler === 'fsrs'}
      <label class="flex flex-col gap-1 text-sm text-[var(--text-muted)] max-w-xs" transition:fade={{ duration: 150 }}>
        Desired retention: {Math.round(settings.fsrs_desired_retention * 100)}%
        <input
          type="range"
          min="70"
          max="99"
          value={Math.round(settings.fsrs_desired_retention * 100)}
          onchange={setDesiredRetention}
          disabled={saving}
        />
        <span class="text-xs">Higher means shorter, more frequent reviews.</span>
      </label>
    {/if}
  </section>

  <section class="flex flex-col gap-3">
    <h2 class="text-lg font-medium text-[var(--text)] m-0">Export / import</h2>
    <div class="flex gap-3">
      <button
        class="rounded-lg px-4 py-2 font-medium border border-[var(--border)] text-[var(--text)] hover:border-[var(--accent)] transition-colors disabled:opacity-40"
        onclick={doExport}
        disabled={exporting}
      >
        {exporting ? 'Exporting…' : 'Export…'}
      </button>
      <button
        class="rounded-lg px-4 py-2 font-medium border border-[var(--border)] text-[var(--text)] hover:border-[var(--accent)] transition-colors disabled:opacity-40"
        onclick={doImport}
        disabled={importing}
      >
        {importing ? 'Importing…' : 'Import…'}
      </button>
    </div>
  </section>

  <section class="flex flex-col gap-3">
    <h2 class="text-lg font-medium text-[var(--text)] m-0">Backups</h2>
    <p class="text-sm text-[var(--text-muted)] m-0">
      Snapshots are taken automatically before every import or restore, plus whenever you make one manually. The
      last 10 are kept.
    </p>
    <button
      class="self-start rounded-lg px-4 py-2 font-medium bg-[var(--accent)] text-[var(--accent-contrast)] hover:bg-[var(--accent-hover)] transition-colors disabled:opacity-40"
      onclick={doCreateBackup}
      disabled={backingUp}
    >
      {backingUp ? 'Backing up…' : 'Create backup now'}
    </button>

    {#if loadingBackups}
      <p class="text-sm text-[var(--text-muted)]">Loading backups…</p>
    {:else if backups.length === 0}
      <p class="text-sm text-[var(--text-muted)]">No backups yet.</p>
    {:else}
      <div class="flex flex-col gap-2" transition:fade={{ duration: 150 }}>
        {#each backups as backup (backup.filename)}
          <div class="flex items-center justify-between rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-2 text-sm">
            <span class="text-[var(--text)]">{formatBackupDate(backup.created_at)}</span>
            <button
              class="text-[var(--text-muted)] hover:text-[var(--accent)] transition-colors disabled:opacity-40"
              onclick={(e) => requestRestore(backup.filename, e)}
              disabled={restoringFilename !== null}
            >
              {restoringFilename === backup.filename ? 'Restoring…' : 'Restore'}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  {#if notice}
    <p class="text-[var(--success)] text-sm" transition:fade={{ duration: 150 }}>{notice}</p>
  {/if}
  {#if error}
    <p class="text-[var(--danger)] text-sm" transition:fade={{ duration: 150 }}>{error}</p>
  {/if}
</div>

{#if confirmRestoreFilename}
  <ConfirmDialog
    title="Restore this backup?"
    message={`This replaces everything currently in your vault with the contents of "${confirmRestoreFilename}". A safety snapshot of the current state is taken first, so this can be undone.`}
    confirmLabel="Restore"
    danger
    onConfirm={confirmRestore}
    onCancel={cancelRestore}
  />
{/if}
