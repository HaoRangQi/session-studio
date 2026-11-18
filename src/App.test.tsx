import { describe, expect, it, vi, beforeEach } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { SourceStatus } from './domain/types';
import App from './App';

const { invokeCommand, openDialog, sourceStatuses, sessionRecords, cleanupCandidateRecords, scanSource, setRoot, resetRoot, previewCleanup, executeCleanup, exportSessions } = vi.hoisted(() => ({
  invokeCommand: vi.fn(),
  openDialog: vi.fn(),
  sourceStatuses: vi.fn(),
  sessionRecords: vi.fn(),
  cleanupCandidateRecords: vi.fn(),
  scanSource: vi.fn(),
  setRoot: vi.fn(),
  resetRoot: vi.fn(),
  previewCleanup: vi.fn(),
  executeCleanup: vi.fn(),
  exportSessions: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: openDialog }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: invokeCommand }));

const statuses: SourceStatus[] = [
  { source: 'claude_code', root: '/Users/tester/.claude/projects', default_root: '/Users/tester/.claude/projects', uses_default_root: true, health: 'needs_validation', last_scan_unix: null, session_count: 0, error: '格式未验证', cli_available: true },
  { source: 'codex', root: '/synthetic/codex-home', default_root: '/Users/tester/.codex', uses_default_root: false, health: 'not_scanned', last_scan_unix: null, session_count: 0, error: null, cli_available: true },
  { source: 'deepseek_harness', root: '/Users/tester/.dsh/sessions', default_root: '/Users/tester/.dsh/sessions', uses_default_root: true, health: 'needs_validation', last_scan_unix: null, session_count: 0, error: '格式未验证', cli_available: false },
  { source: 'pi', root: '/Users/tester/.pi/agent/sessions', default_root: '/Users/tester/.pi/agent/sessions', uses_default_root: true, health: 'ready', last_scan_unix: 1, session_count: 1, error: null, cli_available: true },
];

const session = {
  id: 'pi:one', source: 'pi', source_session_id: 'one', source_root: '/synthetic/pi',
  container_path: '/synthetic/pi/session.jsonl', container_id: 'container-one', title: 'Synthetic session',
  project_path: null, message_count: 3, last_activity_unix: null, modified_at_unix: 1,
  fingerprint: 'fingerprint', candidate_reasons: [],
};

const candidate = { ...session, candidate_reasons: ['unassociated'] };

beforeEach(() => {
  vi.stubGlobal('confirm', vi.fn(() => false));
  vi.clearAllMocks();
  invokeCommand.mockImplementation((command, args = {}) => {
    switch (command) {
      case 'get_sources': return sourceStatuses();
      case 'get_sessions': return sessionRecords();
      case 'get_cleanup_candidates': return cleanupCandidateRecords();
      case 'scan_source': return scanSource(args.source);
      case 'set_source_root': return setRoot(args.source, args.root);
      case 'reset_source_root': return resetRoot(args.source);
      case 'preview_cleanup': return previewCleanup(args.ids);
      case 'execute_cleanup': return executeCleanup(args.token);
      case 'export_sessions': return exportSessions(args.ids, args.format, args.destination);
      case 'clear_index': return Promise.resolve();
      default: throw new Error(`Unexpected command: ${command}`);
    }
  });
  sourceStatuses.mockResolvedValue(statuses);
  sessionRecords.mockResolvedValue([session]);
  cleanupCandidateRecords.mockResolvedValue([candidate]);
  previewCleanup.mockResolvedValue({ token: 'preview', containers: [{ path: session.container_path, session_ids: ['pi:one'], activity: 'unknown', blocked_reason: '活动状态无法确认' }] });
  executeCleanup.mockResolvedValue([{ id: session.container_path, success: false, error: '活动状态无法确认' }]);
  exportSessions.mockResolvedValue([{ id: 'pi:one', success: true, error: null }]);
});

describe('Session Studio workflows', () => {
  it('shows four sources and loads session summaries', async () => {
    render(<App />);
    expect(screen.getByRole('heading', { name: '全部会话' })).toBeTruthy();
    expect(screen.getByText('Claude Code')).toBeTruthy();
    expect(screen.getByText('DeepSeek Harness')).toBeTruthy();
    await waitFor(() => expect(screen.getByText('Synthetic session')).toBeTruthy());
  });

  it('does not mark source roots custom while source status is loading', () => {
    sourceStatuses.mockReturnValue(new Promise<SourceStatus[]>(() => {}));
    render(<App />);
    fireEvent.click(screen.getAllByRole('button', { name: '设置' })[0]);
    expect(screen.queryByRole('button', { name: '恢复默认' })).not.toBeInTheDocument();
    expect(screen.queryByText('自定义目录')).not.toBeInTheDocument();
  });

  it('shows each source root and distinguishes default from custom roots', async () => {
    render(<App />);
    fireEvent.click(screen.getAllByRole('button', { name: '设置' })[0]);
    expect(screen.getByRole('heading', { name: '设置' })).toBeTruthy();
    expect(await screen.findByText('/Users/tester/.claude/projects')).toBeTruthy();
    expect(screen.queryByText(/没有可用的默认目录/)).not.toBeInTheDocument();
    expect(screen.getByText('/synthetic/codex-home')).toBeTruthy();
    expect(screen.getByText('/Users/tester/.dsh/sessions')).toBeTruthy();
    expect(screen.getByText('/Users/tester/.pi/agent/sessions')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: '更改目录' })).toHaveLength(4);
    expect(screen.getAllByRole('button', { name: '扫描' })).toHaveLength(4);
    expect(screen.getAllByRole('button', { name: '恢复默认' })).toHaveLength(1);
    expect(screen.getByText('自定义目录')).toBeTruthy();
    expect(screen.getAllByText('默认目录')).toHaveLength(3);
    expect(screen.getAllByText(/格式未验证/)).toHaveLength(2);
    expect(screen.queryByText(/授权|选择目录/)).not.toBeInTheDocument();
  });

  it('scans a default fallback root without opening the directory picker', async () => {
    const fallbackStatus = { ...statuses[0], root: null, health: 'not_scanned' as const, error: null };
    sourceStatuses.mockResolvedValue([fallbackStatus, ...statuses.slice(1)]);
    scanSource.mockResolvedValue(fallbackStatus);
    render(<App />);
    fireEvent.click(screen.getAllByRole('button', { name: '设置' })[0]);
    await screen.findByText('/Users/tester/.claude/projects');
    const scanButton = screen.getAllByRole('button', { name: '扫描' })[0];
    expect(scanButton).toBeEnabled();
    fireEvent.click(scanButton);
    await waitFor(() => expect(scanSource).toHaveBeenCalledWith('claude_code'));
    expect(setRoot).not.toHaveBeenCalled();
    expect(openDialog).not.toHaveBeenCalled();
  });

  it('restores a custom root through the reset command and refreshes its displayed default', async () => {
    let currentStatuses = [...statuses];
    sourceStatuses.mockImplementation(() => Promise.resolve(currentStatuses));
    resetRoot.mockImplementation(async (source: string) => {
      const restored = { ...statuses[1], root: statuses[1].default_root, uses_default_root: true };
      currentStatuses = currentStatuses.map(item => item.source === source ? restored : item);
      return restored;
    });
    render(<App />);
    fireEvent.click(screen.getAllByRole('button', { name: '设置' })[0]);
    const restoreButton = await screen.findByRole('button', { name: '恢复默认' });
    const codexRow = restoreButton.closest<HTMLElement>('.source-settings-row')!;
    fireEvent.click(restoreButton);
    await waitFor(() => expect(resetRoot).toHaveBeenCalledWith('codex'));
    expect(await within(codexRow).findByText('/Users/tester/.codex')).toBeInTheDocument();
    expect(within(codexRow).getByText('默认目录')).toBeInTheDocument();
    expect(within(codexRow).queryByRole('button', { name: '恢复默认' })).not.toBeInTheDocument();
  });

  it('retains scan command errors after refreshing source state', async () => {
    scanSource.mockRejectedValue(new Error('扫描失败：目录不可访问'));
    render(<App />);
    fireEvent.click(screen.getAllByRole('button', { name: '设置' })[0]);
    await screen.findByText('/Users/tester/.claude/projects');
    const scanButton = screen.getAllByRole('button', { name: '扫描' })[0];
    fireEvent.click(scanButton);
    expect(scanButton).toBeDisabled();
    expect(await screen.findByText(/Error: 扫描失败：目录不可访问/)).toBeInTheDocument();
    await waitFor(() => expect(scanButton).toBeEnabled());
    expect(openDialog).not.toHaveBeenCalled();
  });

  it('loads cleanup candidates from the dedicated candidate query', async () => {
    render(<App />);
    fireEvent.click(screen.getAllByRole('button', { name: '清理候选' })[0]);
    await waitFor(() => expect(cleanupCandidateRecords).toHaveBeenCalled());
    expect(await screen.findByText('Synthetic session')).toBeTruthy();
    expect(screen.getByText('未关联项目')).toBeTruthy();
  });

  it('previews candidate containers and requires confirmation before execution', async () => {
    render(<App />);
    fireEvent.click(screen.getAllByRole('button', { name: '清理候选' })[0]);
    const checkbox = await screen.findByRole('checkbox', { name: '选择 Synthetic session' });
    fireEvent.click(checkbox);
    fireEvent.click(screen.getByRole('button', { name: '移入垃圾篓' }));
    await waitFor(() => expect(previewCleanup).toHaveBeenCalledWith(['pi:one']));
    expect(executeCleanup).not.toHaveBeenCalled();
    vi.mocked(confirm).mockReturnValue(true);
    fireEvent.click(screen.getByRole('button', { name: '移入垃圾篓' }));
    await waitFor(() => expect(executeCleanup).toHaveBeenCalledWith('preview'));
  });
});
