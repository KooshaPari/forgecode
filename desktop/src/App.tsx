import React, { useState, useCallback, useEffect, useRef } from 'react';
import { FiGrid, FiLayout, FiZap, FiSettings, FiPlus, FiSearch, FiX, FiMinimize2, FiMaximize2, FiMinus } from 'react-icons/fi';
import Board from './components/Board';
import SprintView from './components/SprintView';
import IssueCreate from './components/IssueCreate';
import IssueDetail from './components/IssueDetail';
import StatsCard from './components/StatsCard';
import { api } from './store';
import type { Issue, ViewType, BoardStats } from './types';

const navItems: { id: ViewType; label: string; icon: React.ReactNode }[] = [
  { id: 'dashboard', label: 'Dashboard', icon: <FiGrid size={18} /> },
  { id: 'board', label: 'Board', icon: <FiLayout size={18} /> },
  { id: 'sprints', label: 'Sprints', icon: <FiZap size={18} /> },
  { id: 'settings', label: 'Settings', icon: <FiSettings size={18} /> },
];

export default function App() {
  const [currentView, setCurrentView] = useState<ViewType>('board');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [selectedIssue, setSelectedIssue] = useState<Issue | null>(null);
  const [issues, setIssues] = useState<Issue[]>([]);
  const [stats, setStats] = useState<BoardStats | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [isDragging, setIsDragging] = useState(false);
  const dragStartPos = useRef({ x: 0, y: 0 });

  const loadData = useCallback(async () => {
    try {
      const [loadedIssues, loadedStats] = await Promise.all([
        api.getIssues(),
        api.getBoardStats(),
      ]);
      setIssues(loadedIssues);
      setStats(loadedStats);
    } catch (err) {
      console.error('Failed to load data:', err);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    const interval = setInterval(loadData, 2000);
    return () => clearInterval(interval);
  }, [loadData]);

  const handleIssueCreated = useCallback(() => {
    setShowCreateModal(false);
    loadData();
  }, [loadData]);

  const handleIssueUpdated = useCallback(() => {
    setSelectedIssue(null);
    loadData();
  }, [loadData]);

  const handleIssueDeleted = useCallback(() => {
    setSelectedIssue(null);
    loadData();
  }, [loadData]);

  const handleIssueMoved = useCallback(() => {
    loadData();
  }, [loadData]);

  const filteredIssues = issues.filter(issue =>
    searchQuery === '' ||
    issue.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
    issue.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
    issue.assignee.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleTitleBarMouseDown = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('.titlebar-btn')) return;
    setIsDragging(true);
    dragStartPos.current = { x: e.screenX, y: e.screenY };
  };

  const renderView = () => {
    switch (currentView) {
      case 'dashboard':
        return (
          <div className="dashboard-view">
            <div className="dashboard-header">
              <h1>Dashboard</h1>
              <p className="dashboard-subtitle">Project overview and metrics</p>
            </div>
            <div className="stats-grid">
              <StatsCard label="Total Issues" value={stats?.total ?? 0} trend={issues.length > 0 ? 'up' : 'neutral'} />
              <StatsCard label="In Progress" value={stats?.in_progress ?? 0} trend="neutral" />
              <StatsCard label="Completed" value={stats?.done ?? 0} trend={stats && stats.done > stats.backlog ? 'up' : 'down'} />
              <StatsCard label="Backlog" value={stats?.backlog ?? 0} trend="neutral" />
            </div>
            <div className="stats-grid">
              <StatsCard label="Critical" value={stats?.critical ?? 0} trend="neutral" accentColor="var(--color-critical)" />
              <StatsCard label="High" value={stats?.high ?? 0} trend="neutral" accentColor="var(--color-high)" />
              <StatsCard label="Medium" value={stats?.medium ?? 0} trend="neutral" accentColor="var(--color-medium)" />
              <StatsCard label="Low" value={stats?.low ?? 0} trend="neutral" accentColor="var(--color-low)" />
            </div>
            <div className="recent-issues">
              <h2>Recent Issues</h2>
              <div className="recent-list">
                {issues.slice(-5).reverse().map(issue => (
                  <div key={issue.id} className="recent-item" onClick={() => setSelectedIssue(issue)}>
                    <span className={`priority-dot priority-${issue.priority}`} />
                    <span className="recent-title">{issue.title}</span>
                    <span className="recent-status">{issue.status}</span>
                    <span className="recent-assignee">{issue.assignee || 'Unassigned'}</span>
                  </div>
                ))}
                {issues.length === 0 && (
                  <div className="empty-state">
                    <p>No issues yet. Create your first issue to get started.</p>
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      case 'board':
        return <Board issues={filteredIssues} onIssueClick={setSelectedIssue} onIssueMoved={handleIssueMoved} />;
      case 'sprints':
        return <SprintView />;
      case 'settings':
        return (
          <div className="settings-view">
            <h1>Settings</h1>
            <div className="settings-section">
              <h2>Appearance</h2>
              <div className="setting-item">
                <span>Theme</span>
                <span className="setting-value">Dark</span>
              </div>
              <div className="setting-item">
                <span>Compact Mode</span>
                <span className="setting-value">Off</span>
              </div>
            </div>
            <div className="settings-section">
              <h2>About</h2>
              <div className="setting-item">
                <span>Version</span>
                <span className="setting-value">0.1.0</span>
              </div>
              <div className="setting-item">
                <span>Build</span>
                <span className="setting-value">M1</span>
              </div>
            </div>
          </div>
        );
      default:
        return null;
    }
  };

  return (
    <div className="app-container">
      <div className="titlebar" onMouseDown={handleTitleBarMouseDown}>
        <div className="titlebar-title">
          <span className="titlebar-logo">T</span>
          Tracera
        </div>
        <div className="titlebar-btn-group">
          <button className="titlebar-btn"><FiMinus size={14} /></button>
          <button className="titlebar-btn"><FiMaximize2 size={14} /></button>
          <button className="titlebar-btn titlebar-close"><FiX size={14} /></button>
        </div>
      </div>
      <div className="app-layout">
        <aside className="sidebar">
          <nav className="sidebar-nav">
            {navItems.map(item => (
              <button
                key={item.id}
                className={`sidebar-item ${currentView === item.id ? 'active' : ''}`}
                onClick={() => setCurrentView(item.id)}
              >
                {item.icon}
                <span>{item.label}</span>
              </button>
            ))}
          </nav>
          <div className="sidebar-footer">
            <div className="sidebar-user">
              <div className="avatar">TK</div>
              <div className="user-info">
                <span className="user-name">Tracera User</span>
                <span className="user-role">Admin</span>
              </div>
            </div>
          </div>
        </aside>
        <main className="main-content">
          <div className="content-header">
            <div className="search-bar">
              <FiSearch size={16} className="search-icon" />
              <input
                type="text"
                placeholder="Search issues..."
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
              />
              {searchQuery && (
                <button className="search-clear" onClick={() => setSearchQuery('')}>
                  <FiX size={14} />
                </button>
              )}
            </div>
            <button className="btn-primary" onClick={() => setShowCreateModal(true)}>
              <FiPlus size={16} />
              New Issue
            </button>
          </div>
          <div className="content-body">
            {renderView()}
          </div>
        </main>
      </div>
      {showCreateModal && (
        <IssueCreate
          onClose={() => setShowCreateModal(false)}
          onCreated={handleIssueCreated}
        />
      )}
      {selectedIssue && (
        <IssueDetail
          issue={selectedIssue}
          onClose={() => setSelectedIssue(null)}
          onUpdated={handleIssueUpdated}
          onDeleted={handleIssueDeleted}
        />
      )}
    </div>
  );
}
