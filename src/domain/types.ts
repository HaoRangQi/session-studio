export const sourceIds = ['claude_code', 'codex', 'deepseek_harness', 'pi'] as const;
export type SourceId = (typeof sourceIds)[number];

export type SourceHealth =
  | 'not_scanned'
  | 'missing'
  | 'needs_validation'
  | 'scanning'
  | 'ready'
  | 'empty'
  | 'permission_denied'
  | 'unsupported'
  | 'failed';

export type CandidateReason = 'missing_project' | 'unassociated' | 'inactive90_days';
export type ActivityCertainty = 'running' | 'inactive' | 'unknown';

export interface SourceStatus {
  source: SourceId;
  root: string | null;
  default_root: string | null;
  uses_default_root: boolean;
  health: SourceHealth;
  last_scan_unix: number | null;
  session_count: number;
  error: string | null;
  cli_available: boolean;
}

export interface SessionSummary {
  id: string;
  source: SourceId;
  source_session_id: string;
  source_root: string;
  container_path: string;
  container_id: string;
  title: string | null;
  project_path: string | null;
  message_count: number | null;
  last_activity_unix: number | null;
  modified_at_unix: number;
  fingerprint: string;
  candidate_reasons: CandidateReason[];
}

export interface ActivityRecord {
  pid: number;
  source: SourceId;
  project_path: string | null;
  started_at_unix: number | null;
  state: string;
  child_pids: number[];
}

export interface ActivitySnapshot {
  records: ActivityRecord[];
  reliable: boolean;
  error: string | null;
}

export interface CleanupContainer {
  path: string;
  session_ids: string[];
  activity: ActivityCertainty;
  blocked_reason: string | null;
}

export interface CleanupPreview {
  token: string;
  containers: CleanupContainer[];
}

export interface ItemOutcome {
  id: string;
  success: boolean;
  error: string | null;
}

export interface Message {
  role: string;
  timestamp: string | null;
  content: unknown;
  attachments: { kind: string; reference: string }[];
}

export interface ParsedSession {
  summary: SessionSummary;
  messages: Message[];
}

export interface AnalysisPreview {
  token: string;
  source: SourceId;
  input: string;
  executable: string;
  arguments: string[];
  backend: string;
  read_only_guaranteed: boolean;
  ephemeral: boolean;
  warnings: string[];
}

export interface AnalysisTask {
  id: string;
  source: SourceId;
  status: string;
  created_at_unix: number;
  input: string;
  stdout: string;
  stderr: string;
  exit_code: number | null;
  error: string | null;
}
