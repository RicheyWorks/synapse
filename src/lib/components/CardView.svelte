<script lang="ts">
  import type { CardContent } from '../api'

  let { card, revealed }: { card: CardContent; revealed: boolean } = $props()

  interface ClozePart {
    text: string
    isBlank: boolean
  }

  function renderCloze(text: string, reveal: boolean): ClozePart[] {
    const parts: ClozePart[] = []
    const re = /\{\{c\d+::(.*?)\}\}/g
    let last = 0
    let match: RegExpExecArray | null
    while ((match = re.exec(text))) {
      if (match.index > last) parts.push({ text: text.slice(last, match.index), isBlank: false })
      parts.push({ text: reveal ? match[1] : '…', isBlank: true })
      last = match.index + match[0].length
    }
    if (last < text.length) parts.push({ text: text.slice(last), isBlank: false })
    return parts
  }
</script>

{#if card.type === 'basic'}
  {#if revealed}
    <p class="text-[var(--text)]">{card.answer}</p>
  {/if}
{:else if card.type === 'cloze'}
  <p class="text-[var(--text)] leading-relaxed">
    {#each renderCloze(card.text, revealed) as part, i (i)}
      {#if part.isBlank}
        <span
          class="px-1.5 rounded font-medium"
          style={revealed
            ? 'background: color-mix(in srgb, var(--accent) 20%, transparent); color: var(--accent);'
            : 'background: var(--bg-inset); color: var(--text-muted);'}
        >
          {part.text}
        </span>
      {:else}
        {part.text}
      {/if}
    {/each}
  </p>
{:else if card.type === 'code'}
  {#if revealed}
    <div class="w-full text-left">
      <div class="text-xs uppercase tracking-wide text-[var(--text-muted)] mb-1">{card.language || 'code'}</div>
      <pre class="rounded-lg bg-[var(--bg-inset)] p-3 overflow-x-auto text-sm text-[var(--text)]"><code>{card.code}</code></pre>
    </div>
  {/if}
{:else if card.type === 'image'}
  {#if revealed}
    <div class="flex flex-col items-center gap-2">
      <img src={card.path} alt={card.caption ?? ''} class="max-w-full max-h-64 rounded-lg border border-[var(--border)]" />
      {#if card.caption}<p class="text-sm text-[var(--text-muted)]">{card.caption}</p>{/if}
    </div>
  {/if}
{/if}
