import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { isTauri } from '@tauri-apps/api/core';
import { Activity, Archive, ArrowDownToLine, FolderKanban, FolderOpen, ListFilter, Search, Settings2, Sparkles, Trash2 } from 'lucide-react';
import { appCommands } from './lib/commands';
import type { SessionSummary, SourceId, SourceStatus } from './domain/types';
import './styles.css';

const sources: { id: SourceId; name: string; initials: string }[] = [
  { id: 'claude_code', name: 'Claude Code', initials: 'CC' },
  { id: 'codex', name: 'Codex', initials: 'CX' },
  { id: 'deepseek_harness', name: 'DeepSeek Harness', initials: 'DS' },
  { id: 'pi', name: 'Pi', initials: 'PI' },
];

const navigation = [
  { id: 'sessions', label: '全部会话', icon: Archive },
  { id: 'projects', label: '项目', icon: FolderKanban },
  { id: 'cleanup', label: '清理候选', icon: Trash2 },
  { id: 'activity', label: '活动任务', icon: Activity },
] as const;

const healthLabels: Record<string, string> = {
  not_scanned: '尚未扫描', missing: '目录不存在', needs_validation: '待验证', scanning: '扫描中', ready: '可用', empty: '扫描完成，无会话',
  permission_denied: '权限不足', unsupported: '格式不支持', failed: '扫描失败',
};

export default function App() {
  const [active, setActive] = useState<(typeof navigation)[number]['id'] | 'settings'>('sessions');
  const [selectedSource, setSelectedSource] = useState<SourceId | 'all'>('all');
  const [sourcesStatus, setSourcesStatus] = useState<SourceStatus[]>([]);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [status, setStatus] = useState<string>('尚未扫描');
  const [busy, setBusy] = useState(false);
  const current = navigation.find(item => item.id === active);

  async function refresh() {
    if (!isTauri()) return;
    try {
      const sessionQuery = selectedSource === 'all' ? undefined : selectedSource;
      const [sourceStates, records] = await Promise.all([
        appCommands.sources(),
        active === 'cleanup' ? appCommands.cleanupCandidates(sessionQuery) : appCommands.sessions(sessionQuery),
      ]);
      setSourcesStatus(sourceStates);
      setSessions(records);
      const source = sourceStates.find(item => item.source === selectedSource);
      setStatus(source ? healthLabels[source.health] : '尚未扫描');
    } catch (error) { setStatus(String(error)); }
  }

  useEffect(() => { void refresh(); }, [selectedSource, active]);

  async function chooseRoot(source: SourceId) {
    try {
      const selectedPath = await open({ directory: true, multiple: false, title: `选择 ${sourceName(source)} 会话目录` });
      if (!selectedPath || Array.isArray(selectedPath)) return;
      setBusy(true);
      const result = await appCommands.setRoot(source, selectedPath);
      setStatus(`${sourceName(source)}：${healthLabels[result.health]}`);
      await refresh();
    } catch (error) { setStatus(String(error)); }
    finally { setBusy(false); }
  }

  async function scan(source: SourceId) {
    setBusy(true);
    try {
      const result = await appCommands.scan(source);
      setStatus(`${sourceName(source)}：${healthLabels[result.health]}，${result.session_count} 条会话`);
      await refresh();
    } catch (error) {
      await refresh();
      setStatus(String(error));
    }
    finally { setBusy(false); }
  }

  async function restoreDefaultRoot(source: SourceId) {
    setBusy(true);
    try {
      const result = await appCommands.resetRoot(source);
      setSourcesStatus(current => current.map(item => item.source === source ? result : item));
      setStatus(`${sourceName(source)}：${healthLabels[result.health]}`);
    } catch (error) { setStatus(String(error)); }
    finally { setBusy(false); }
  }

  function sourceName(id: SourceId) { return sources.find(source => source.id === id)?.name ?? id; }
  function toggleSession(id: string) { setSelected(value => value.includes(id) ? value.filter(item => item !== id) : [...value, id]); }

  async function exportSelected(format: 'markdown' | 'json') {
    const destination = await open({ directory: true, multiple: false, title: '选择导出目录' });
    if (!destination || Array.isArray(destination)) return;
    try {
      const results = await appCommands.export(selected, format, destination);
      setStatus(`导出完成：${results.filter(item => item.success).length}/${results.length} 条`);
    } catch (error) { setStatus(String(error)); }
  }

  async function cleanupSelected() {
    try {
      const preview = await appCommands.cleanupPreview(selected);
      const list = preview.containers.map(container => `${container.path}\n${container.session_ids.length} 条会话；${container.activity}${container.blocked_reason ? `；阻止：${container.blocked_reason}` : ''}`).join('\n\n');
      if (!confirm(`清理预览（仅移入 macOS 垃圾篓）\n\n${list}\n\n继续处理可执行项？`)) return;
      const results = await appCommands.cleanup(preview.token);
      setStatus(`清理结果：${results.filter(item => item.success).length}/${results.length} 个容器已移入垃圾篓`);
      await refresh();
    } catch (error) { setStatus(String(error)); }
  }

  const currentSource = sourcesStatus.find(item => item.source === selectedSource);
  const currentRoot = currentSource?.root ?? currentSource?.default_root;
  const visibleSessions = sessions;
  const projectView = active === 'projects';
  const grouped = projectView ? visibleSessions.reduce<Record<string, SessionSummary[]>>((groups, session) => {
    const key = session.project_path ?? '未关联项目';
    (groups[key] ??= []).push(session);
    return groups;
  }, {}) : null;
  const activityPage = active === 'activity';

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="主导航">
        <div className="brand"><div className="brand-mark">S</div><span>Session Studio</span></div>
        <div className="sidebar-section-label">工作区</div>
        <nav className="primary-nav">{navigation.map(item => { const Icon = item.icon; return <button key={item.id} className={`nav-item ${active === item.id ? 'active' : ''}`} onClick={() => setActive(item.id)}><Icon size={17} strokeWidth={1.8} /><span>{item.label}</span>{item.id === 'sessions' && <span className="nav-count">{sessions.length}</span>}</button>; })}</nav>
        <div className="source-heading"><span>来源</span><button aria-label="来源设置" title="来源设置" onClick={() => setActive('settings')}><Settings2 size={15} /></button></div>
        <div className="source-list">{sources.map(source => { const state = sourcesStatus.find(item => item.source === source.id); return <button key={source.id} className={`source-item ${selectedSource === source.id ? 'source-selected' : ''}`} onClick={() => { setSelectedSource(source.id); setActive('sessions'); }}><span className={`source-logo ${source.id}`}>{source.initials}</span><span className="source-name">{source.name}</span><span className={`source-dot health-${state?.health ?? 'not_scanned'}`} title={healthLabels[state?.health ?? 'not_scanned']} /></button>; })}</div>
        <div className="sidebar-bottom"><button className={`nav-item ${active === 'settings' ? 'active' : ''}`} onClick={() => setActive('settings')}><Settings2 size={17} /><span>设置</span></button></div>
      </aside>
      <main className="main-panel">
        <header className="topbar"><div className="breadcrumbs"><span>工作区</span><span className="crumb-separator">/</span><strong>{current?.label ?? '设置'}</strong></div><div className="top-actions"><span className="local-indicator"><span />仅本机</span><button className="icon-button" aria-label="搜索" title="搜索"><Search size={17} /></button><button className="icon-button" aria-label="筛选" title="筛选"><ListFilter size={17} /></button></div></header>
        <section className="content">
          <div className="page-heading"><div><div className="eyebrow">SESSION LIBRARY</div><h1>{current?.label ?? '设置'}</h1><p>跨来源查看会话索引。会话正文仅在打开、导出或分析时按需读取。</p></div><div className="heading-actions">{selected.length > 0 && <><button className="button secondary" onClick={() => void exportSelected('markdown')}><ArrowDownToLine size={15} />导出 Markdown</button><button className="button secondary" onClick={() => void exportSelected('json')}>JSON</button><button className="button secondary" onClick={() => void cleanupSelected()}><Trash2 size={15} />移入垃圾篓</button></>}<button className="button primary" disabled={busy || (selectedSource !== 'all' && !currentSource?.root && !currentSource?.default_root)} onClick={() => selectedSource === 'all' ? setActive('settings') : scan(selectedSource)}><ArrowDownToLine size={16} />{selectedSource === 'all' ? '管理来源' : '扫描来源'}</button></div></div>
          <div className="source-status-strip" role="status"><span className="status-icon"><FolderOpen size={17} /></span><div className="status-text"><strong>{currentSource ? healthLabels[currentSource.health] : '来源状态'}</strong><span>{currentSource ? `${status}${currentRoot ? ` · ${currentRoot}` : '。没有可用的默认目录，请更改目录。'}` : `${status}。默认路径见来源设置。`}</span></div><button className="text-button" onClick={() => setActive('settings')}>管理来源 <span>→</span></button></div>
          {active === 'settings' ? <section className="settings-view"><div className="section-header"><div><h2>来源状态</h2><p>默认目录可直接扫描；应用不会自动扫描主目录或隐藏路径。</p></div></div><div className="source-settings-list">{sources.map(source => { const state = sourcesStatus.find(item => item.source === source.id); const root = state?.root ?? state?.default_root; return <div className="source-settings-row" key={source.id}><div className={`source-logo ${source.id}`}>{source.initials}</div><div className="source-settings-info"><strong>{source.name}</strong><span className="source-root-kind">{state ? (state.uses_default_root ? '默认目录' : '自定义目录') : '加载中'}</span><code className="source-root-path">{root ?? '默认目录不可用'}</code><span>{healthLabels[state?.health ?? 'not_scanned']}{state?.session_count ? ` · ${state.session_count} 条` : ''}{state?.error ? ` · ${state.error}` : ''}</span></div><button className="button secondary" disabled={busy} onClick={() => void chooseRoot(source.id)}><FolderOpen size={15} />更改目录</button>{state && !state.uses_default_root && <button className="button secondary" disabled={busy || !state.default_root} onClick={() => void restoreDefaultRoot(source.id)}>恢复默认</button>}<button className="button primary" disabled={busy || !root} onClick={() => void scan(source.id)}>扫描</button></div>; })}</div><div className="index-management"><div><strong>本地元数据索引</strong><span>清空索引不会删除来源文件或分析任务记录。</span></div><button className="button secondary" onClick={async () => { if (confirm('清空本地会话元数据索引？此操作不会删除来源文件。')) { await appCommands.clearIndex(); setStatus('索引已清空'); await refresh(); } }}>清空索引</button></div></section>
          : active === 'cleanup' || active === 'projects' || active === 'sessions' ? <>
            <div className="toolbar-row"><div className="segmented" aria-label="会话范围"><button className={`segment ${!projectView ? 'active' : ''}`} onClick={() => setActive('sessions')}>全部会话</button><button className={`segment ${projectView ? 'active' : ''}`} onClick={() => setActive('projects')}>按项目</button></div><div className="filter-summary">{selectedSource === 'all' ? '所有来源' : sourceName(selectedSource)}<span className="filter-divider" />{active === 'cleanup' ? '清理候选' : '最近活动'}</div></div>
            {visibleSessions.length === 0 ? <div className="empty-workspace"><div className="empty-illustration"><div className="file-sheet"><div className="sheet-top"><span /><span /></div><div className="sheet-line long" /><div className="sheet-line" /><div className="sheet-line short" /><div className="sheet-code">{'{ }'}</div></div><div className="empty-orbit orbit-one" /><div className="empty-orbit orbit-two" /><div className="empty-cross">+</div><div className="empty-dot" /></div><h2>{active === 'cleanup' ? '没有清理候选' : '还没有会话'}</h2><p>{active === 'cleanup' ? '满足已验证候选条件的会话会出现在这里。活动关联不确定时将始终阻止清理。' : '来源目录可用后可直接扫描，索引只保存来源、项目和时间等元数据。'}</p><button className="button primary" onClick={() => setActive('settings')}><Settings2 size={16} />管理来源</button><div className="empty-footnote"><span className="lock-symbol">⌑</span>不会读取整个主目录，也不会复制会话正文</div></div>
            : projectView ? <div className="session-list">{Object.entries(grouped ?? {}).map(([project, records]) => <section className="project-group" key={project}><h2>{project}</h2>{records?.map(session => <SessionRow key={session.id} session={session} selected={selected.includes(session.id)} onSelect={() => toggleSession(session.id)} />)}</section>)}</div>
            : <div className="session-list">{visibleSessions.map(session => <SessionRow key={session.id} session={session} selected={selected.includes(session.id)} onSelect={() => toggleSession(session.id)} />)}</div>}
          </> : activityPage ? <div className="disabled-page"><Activity size={22} /><h2>活动监控尚未启用</h2><p>当前构建没有可靠的进程到会话关联实现。清理流程会将所有容器标记为关联未知并阻止移入垃圾篓。</p></div> : <div className="disabled-page"><Sparkles size={22} /><h2>该功能尚未启用</h2><p>当前仅实现了 Pi、Codex 的格式化样本解析与本地索引；Claude Code、DeepSeek Harness 和 CLI 分析仍待验证。</p></div>}
          <div className="footer-status"><span><span className="footer-dot" />本地索引就绪</span><span>索引范围：仅元数据</span></div>
        </section>
      </main>
    </div>
  );
}

function SessionRow({ session, selected, onSelect }: { session: SessionSummary; selected: boolean; onSelect: () => void }) {
  const source = sources.find(item => item.id === session.source)!;
  const modified = new Date(session.modified_at_unix * 1000).toLocaleString();
  return <article className="session-row"><input type="checkbox" aria-label={`选择 ${session.title ?? session.source_session_id}`} checked={selected} onChange={onSelect} /><span className={`source-logo ${session.source}`}>{source.initials}</span><div className="session-info"><strong>{session.title ?? '未命名会话'}</strong><span>{session.project_path ?? '未关联项目'} · {session.message_count ?? '未知'} 条消息</span></div><div className="session-meta"><time>{modified}</time><span>{source.name}</span></div>{session.candidate_reasons.map(reason => <span className="candidate-tag" key={reason}>{reason === 'missing_project' ? '项目不存在' : reason === 'unassociated' ? '未关联项目' : '90 天未活动'}</span>)}</article>;
}
