<script lang="ts">
  import { onMount } from 'svelte'
  import Dashboard from './lib/components/Dashboard.svelte'
  import ReviewSession from './lib/components/ReviewSession.svelte'
  import TrackManager from './lib/components/TrackManager.svelte'
  import SettingsPanel from './lib/components/SettingsPanel.svelte'
  import { api, describeError, type Settings } from './lib/api'

  type View = 'dashboard' | 'review' | 'tracks' | 'settings'

  let view = $state<View>('dashboard')
  let settings = $state<Settings>({ daily_review_limit: 20, theme: 'neural' })
  let loadError = $state<string | null>(null)

  onMount(async () => {
    try {
      settings = await api.getSettings()
    } catch (e) {
      loadError = describeError(e)
    }
  })

  $effect(() => {
    document.documentElement.dataset.theme = settings.theme
  })

  const navItems: { id: View; label: string }[] = [
    { id: 'dashboard', label: 'Dashboard' },
    { id: 'tracks', label: 'Tracks' },
    { id: 'settings', label: 'Settings' },
  ]
</script>

{#if view === 'review'}
  <ReviewSession onFinish={() => (view = 'dashboard')} />
{:else}
  <div class="flex flex-col h-full">
    <nav class="flex items-center justify-between border-b border-[var(--border)] px-6 py-3">
      <span class="font-semibold text-[var(--text)]">Synapse</span>
      <div class="flex gap-1">
        {#each navItems as item (item.id)}
          <button
            class={`rounded-lg px-3 py-1.5 text-sm transition-colors ${
              view === item.id ? 'bg-[var(--bg-inset)] text-[var(--text)]' : 'text-[var(--text-muted)]'
            }`}
            onclick={() => (view = item.id)}
          >
            {item.label}
          </button>
        {/each}
      </div>
    </nav>
    <main class="flex-1 overflow-y-auto">
      {#if loadError}
        <p class="text-[var(--danger)] p-8">{loadError}</p>
      {:else if view === 'dashboard'}
        <Dashboard onStartReview={() => (view = 'review')} />
      {:else if view === 'tracks'}
        <TrackManager />
      {:else if view === 'settings'}
        <SettingsPanel {settings} onChange={(s) => (settings = s)} />
      {/if}
    </main>
  </div>
{/if}
