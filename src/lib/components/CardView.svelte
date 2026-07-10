<script lang="ts">
  import Prism from 'prismjs'
  import 'prismjs/components/prism-clike'
  import 'prismjs/components/prism-javascript'
  import 'prismjs/components/prism-typescript'
  import 'prismjs/components/prism-python'
  import 'prismjs/components/prism-bash'
  import 'prismjs/components/prism-json'
  import 'prismjs/components/prism-css'
  import 'prismjs/components/prism-markup'
  import 'prismjs/components/prism-rust'
  import 'prismjs/components/prism-sql'
  import type { CardContent } from '../api'
  import { colorForIndex } from '../colors'

  let { card, revealed }: { card: CardContent; revealed: boolean } = $props()

  interface ClozePart {
    text: string
    isBlank: boolean
    clozeNum?: number
    hint?: string | null
  }

  // Anki-style cloze: {{c1::answer}} or {{c1::answer::hint}}.
  function renderCloze(text: string, reveal: boolean): ClozePart[] {
    const parts: ClozePart[] = []
    const re = /\{\{c(\d+)::(.*?)\}\}/g
    let last = 0
    let match: RegExpExecArray | null
    while ((match = re.exec(text))) {
      if (match.index > last) parts.push({ text: text.slice(last, match.index), isBlank: false })
      const clozeNum = Number(match[1])
      const raw = match[2]
      const sep = raw.indexOf('::')
      const answer = sep === -1 ? raw : raw.slice(0, sep)
      const hint = sep === -1 ? null : raw.slice(sep + 2)
      parts.push({ text: reveal ? answer : hint ? `[${hint}]` : '…', isBlank: true, clozeNum, hint })
      last = match.index + match[0].length
    }
    if (last < text.length) parts.push({ text: text.slice(last), isBlank: false })
    return parts
  }

  // More than one distinct cloze number in a card: color-tag each number so
  // multi-blank clozes are easier to tell apart. A single-number card (the
  // common case) skips this — no point tinting a card with only {{c1::}}.
  function distinctClozeNums(text: string): number[] {
    const nums = new Set<number>()
    for (const part of renderCloze(text, false)) {
      if (part.isBlank && part.clozeNum !== undefined) nums.add(part.clozeNum)
    }
    return [...nums].sort((a, b) => a - b)
  }

  const LANG_ALIASES: Record<string, string> = {
    js: 'javascript',
    ts: 'typescript',
    sh: 'bash',
    shell: 'bash',
    html: 'markup',
    htm: 'markup',
    xml: 'markup',
    py: 'python',
    rs: 'rust',
  }

  function normalizeLang(lang: string): string {
    const l = (lang || '').toLowerCase().trim()
    return LANG_ALIASES[l] ?? l
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
  }

  function highlightCode(code: string, language: string): string {
    const key = normalizeLang(language)
    const grammar = Prism.languages[key]
    if (!grammar) return escapeHtml(code)
    return Prism.highlight(code, grammar, key)
  }
</script>

{#if card.type === 'basic'}
  {#if revealed}
    <p class="text-[var(--text)]">{card.answer}</p>
  {/if}
{:else if card.type === 'cloze'}
  {@const nums = distinctClozeNums(card.text)}
  <p class="text-[var(--text)] leading-relaxed">
    {#each renderCloze(card.text, revealed) as part, i (i)}
      {#if part.isBlank}
        <span
          class="px-1.5 rounded font-medium"
          style={revealed
            ? 'background: color-mix(in srgb, var(--accent) 20%, transparent); color: var(--accent);'
            : 'background: var(--bg-inset); color: var(--text-muted);'}
          title={part.hint && !revealed ? `Hint for c${part.clozeNum}` : undefined}
        >
          {#if nums.length > 1 && part.clozeNum !== undefined}
            <span
              class="inline-block w-1.5 h-1.5 rounded-full mr-1 align-middle"
              style={`background: ${colorForIndex(nums.indexOf(part.clozeNum))}`}
              aria-hidden="true"
            ></span>
          {/if}
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
      <pre class="prism-block rounded-lg bg-[var(--bg-inset)] p-3 overflow-x-auto text-sm"><code
          class={`language-${normalizeLang(card.language)}`}>{@html highlightCode(card.code, card.language)}</code
        ></pre>
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
