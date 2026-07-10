import { invoke as tauriInvoke } from '@tauri-apps/api/tauri'

export interface ReviewLogEntry {
  reviewed_at: string
  score: number
  interval_before_days: number
  interval_after_days: number
  ease_factor_after: number
}

export type CardContent =
  | { type: 'basic'; answer: string }
  | { type: 'cloze'; text: string }
  | { type: 'code'; language: string; code: string }
  | { type: 'image'; path: string; caption: string | null }

export interface MemoryItem {
  id: string
  training_track: string
  prompt: string
  card: CardContent
  repetitions: number
  ease_factor: number
  interval_days: number
  next_review: string
  created_at: string
  review_log: ReviewLogEntry[]
  lapses: number
  total_lapses: number
  related_ids: string[]
}

export interface Stats {
  total_items: number
  due_now: number
  total_reviews: number
  retention_rate: number
  current_streak_days: number
  best_streak_days: number
  average_ease: number
  leech_count: number
}

export interface TrackSummary {
  name: string
  total: number
  due: number
}

export type ThemeId = 'neural' | 'blackbeard'

export interface Settings {
  daily_review_limit: number
  theme: string
}

export interface SynapseError {
  kind: 'NotFound' | 'Io' | 'Serialization' | 'InvalidOperation'
  message: string
}

export interface GraphNode {
  id: string
  label: string
  track: string
}

export interface GraphEdge {
  source: string
  target: string
}

export interface KnowledgeGraph {
  nodes: GraphNode[]
  edges: GraphEdge[]
}

export interface HeatmapDay {
  date: string
  review_count: number
}

export interface RetentionPoint {
  date: string
  reviews: number
  retention_rate: number
}

export interface Achievement {
  id: string
  title: string
  description: string
  unlocked: boolean
}

export interface GamificationSummary {
  xp: number
  level: number
  title: string
  achievements: Achievement[]
}

export interface BackupInfo {
  filename: string
  created_at: string
}

/** Thin wrapper so call sites don't import from @tauri-apps/api directly. */
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return tauriInvoke<T>(cmd, args)
}

export const api = {
  addMemory: (track: string, prompt: string, card: CardContent) =>
    invoke<MemoryItem>('add_memory', { track, prompt, card }),

  reviewMemory: (id: string, score: number) => invoke<MemoryItem>('review_memory', { id, score }),

  getDueMemories: () => invoke<MemoryItem[]>('get_due_memories'),

  getAllMemories: () => invoke<MemoryItem[]>('get_all_memories'),

  getStats: () => invoke<Stats>('get_stats'),

  listTracks: () => invoke<TrackSummary[]>('list_all_tracks'),

  startReviewSession: (limit?: number) => invoke<MemoryItem[]>('start_review_session', { limit }),

  getSettings: () => invoke<Settings>('get_settings'),

  updateSettings: (settings: Settings) => invoke<Settings>('update_settings', { settings }),

  exportMemories: (path: string) => invoke<void>('export_memories', { path }),

  importMemories: (path: string) => invoke<number>('import_memories', { path }),

  getKnowledgeGraph: () => invoke<KnowledgeGraph>('get_knowledge_graph'),

  // Rust params are id_a/id_b; Tauri v1 camelCases multi-word arg names for JS.
  linkMemories: (idA: string, idB: string) => invoke<void>('link_memories', { idA, idB }),

  unlinkMemories: (idA: string, idB: string) => invoke<void>('unlink_memories', { idA, idB }),

  getReviewHeatmap: (days: number) => invoke<HeatmapDay[]>('get_review_heatmap', { days }),

  getRetentionCurve: () => invoke<RetentionPoint[]>('get_retention_curve'),

  getForgettingCurve: (id: string, daysAhead: number) =>
    invoke<[number, number][]>('get_forgetting_curve', { id, daysAhead }),

  getHardestItems: (limit: number) => invoke<MemoryItem[]>('get_hardest_items', { limit }),

  getGamification: () => invoke<GamificationSummary>('get_gamification'),

  createManualBackup: () => invoke<string>('create_manual_backup'),

  listBackups: () => invoke<BackupInfo[]>('list_backups'),

  restoreBackup: (filename: string) => invoke<number>('restore_backup', { filename }),
}

export function isSynapseError(e: unknown): e is SynapseError {
  return typeof e === 'object' && e !== null && 'kind' in e && 'message' in e
}

export function describeError(e: unknown): string {
  if (isSynapseError(e)) return e.message
  if (e instanceof Error) return e.message
  return String(e)
}
