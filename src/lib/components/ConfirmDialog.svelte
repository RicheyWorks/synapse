<script lang="ts">
  import { fade, scale } from 'svelte/transition'

  let {
    title,
    message,
    confirmLabel = 'Confirm',
    danger = false,
    onConfirm,
    onCancel,
  }: {
    title: string
    message: string
    confirmLabel?: string
    danger?: boolean
    onConfirm: () => void
    onCancel: () => void
  } = $props()

  let dialogEl = $state<HTMLDivElement | null>(null)
  let confirmBtn = $state<HTMLButtonElement | null>(null)

  $effect(() => {
    confirmBtn?.focus()
  })

  // No backdrop-click-to-close by design: this is used for destructive
  // confirmations, so closing should be a deliberate Cancel/Escape, not a
  // stray click.
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault()
      onCancel()
      return
    }
    if (e.key !== 'Tab' || !dialogEl) return
    const focusables = dialogEl.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    )
    if (focusables.length === 0) return
    const first = focusables[0]
    const last = focusables[focusables.length - 1]
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault()
      first.focus()
    }
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" transition:fade={{ duration: 120 }}>
  <div
    bind:this={dialogEl}
    class="w-full max-w-sm rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-6 flex flex-col gap-4"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-dialog-title"
    aria-describedby="confirm-dialog-message"
    tabindex="-1"
    onkeydown={handleKeydown}
    transition:scale={{ duration: 120, start: 0.96 }}
  >
    <h2 id="confirm-dialog-title" class="text-lg font-medium text-[var(--text)] m-0">{title}</h2>
    <p id="confirm-dialog-message" class="text-sm text-[var(--text-muted)] m-0">{message}</p>
    <div class="flex justify-end gap-2">
      <button
        class="rounded-lg px-4 py-2 text-sm font-medium border border-[var(--border)] text-[var(--text)] hover:border-[var(--accent)] transition-colors"
        onclick={onCancel}
      >
        Cancel
      </button>
      <button
        bind:this={confirmBtn}
        class={`rounded-lg px-4 py-2 text-sm font-medium transition-colors ${
          danger
            ? 'bg-[var(--danger)] text-white hover:opacity-90'
            : 'bg-[var(--accent)] text-[var(--accent-contrast)] hover:bg-[var(--accent-hover)]'
        }`}
        onclick={onConfirm}
      >
        {confirmLabel}
      </button>
    </div>
  </div>
</div>
