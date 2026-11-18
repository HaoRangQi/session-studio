import { invoke } from '@tauri-apps/api/core';
import type {
  ActivitySnapshot,
  AnalysisPreview,
  AnalysisTask,
  CleanupPreview,
  ItemOutcome,
  ParsedSession,
  SessionSummary,
  SourceId,
  SourceStatus,
} from '../domain/types';

export const appCommands = {
  sources: () => invoke<SourceStatus[]>('get_sources'),
  sessions: (source?: SourceId) => invoke<SessionSummary[]>('get_sessions', { source: source ?? null }),
  cleanupCandidates: (source?: SourceId) => invoke<SessionSummary[]>('get_cleanup_candidates', { source: source ?? null }),
  scan: (source: SourceId) => invoke<SourceStatus>('scan_source', { source }),
  setRoot: (source: SourceId, root: string) => invoke<SourceStatus>('set_source_root', { source, root }),
  resetRoot: (source: SourceId) => invoke<SourceStatus>('reset_source_root', { source }),
  parseSession: (id: string) => invoke<ParsedSession>('parse_session', { id }),
  activity: () => invoke<ActivitySnapshot>('get_activity'),
  cleanupPreview: (ids: string[]) => invoke<CleanupPreview>('preview_cleanup', { ids }),
  cleanup: (token: string) => invoke<ItemOutcome[]>('execute_cleanup', { token }),
  export: (ids: string[], format: 'markdown' | 'json', destination: string) => invoke<ItemOutcome[]>('export_sessions', { ids, format, destination }),
  analysisPreview: (source: SourceId, instruction: string, selections: { session_id: string; start: number; end: number | null }[]) =>
    invoke<AnalysisPreview>('preview_analysis', { source, instruction, selections }),
  runAnalysis: (token: string, approvedExternal: boolean, approvedWritable: boolean, approvedPersistent: boolean) =>
    invoke<AnalysisTask>('run_analysis', { token, approvedExternal, approvedWritable, approvedPersistent }),
  tasks: () => invoke<AnalysisTask[]>('get_tasks'),
  deleteTask: (id: string) => invoke<void>('delete_task', { id }),
  clearIndex: () => invoke<void>('clear_index'),
};
