import React, { useState, useEffect, useCallback } from 'react';
import { FiPlus, FiPlay, FiCheck, FiCalendar } from 'react-icons/fi';
import { api } from '../store';
import type { SprintProgress } from '../types';

export default function SprintView() {
  const [sprints, setSprints] = useState<SprintProgress[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [name, setName] = useState('');
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [goal, setGoal] = useState('');
  const [isCreating, setIsCreating] = useState(false);

  const loadSprints = useCallback(async () => {
    try {
      const data = await api.getSprintProgress();
      setSprints(data);
    } catch (err) {
      console.error('Failed to load sprints:', err);
    }
  }, []);

  useEffect(() => {
    loadSprints();
    const interval = setInterval(loadSprints, 3000);
    return () => clearInterval(interval);
  }, [loadSprints]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setIsCreating(true);
    try {
      await api.createSprint({
        name: name.trim(),
        start_date: startDate || new Date().toISOString().split('T')[0],
        end_date: endDate || new Date(Date.now() + 14 * 86400000).toISOString().split('T')[0],
        goal: goal.trim(),
      });
      setName('');
      setStartDate('');
      setEndDate('');
      setGoal('');
      setShowCreateModal(false);
      loadSprints();
    } catch (err) {
      console.error('Failed to create sprint:', err);
    } finally {
      setIsCreating(false);
    }
  };

  const handleActivate = async (id: string) => {
    try {
      await api.activateSprint(id);
      loadSprints();
    } catch (err) {
      console.error('Failed to activate sprint:', err);
    }
  };

  const formatDate = (d: string) => {
    try {
      return new Date(d).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    } catch {
      return d;
    }
  };

  return (
    <div className="sprint-view">
      <div className="sprint-header">
        <div>
          <h1>Sprint Management</h1>
          <p className="sprint-subtitle">Track sprints, velocity, and progress</p>
        </div>
        <button className="btn-primary" onClick={() => setShowCreateModal(true)}>
          <FiPlus size={16} />
          New Sprint
        </button>
      </div>

      <div className="sprint-list">
        {sprints.map(sp => (
          <div key={sp.sprint.id} className={`sprint-card ${sp.sprint.active ? 'sprint-active' : ''}`}>
            <div className="sprint-card-header">
              <div className="sprint-card-title-group">
                {sp.sprint.active && <span className="sprint-badge active">Active</span>}
                <h3>{sp.sprint.name}</h3>
              </div>
              {!sp.sprint.active && (
                <button className="btn-secondary btn-sm" onClick={() => handleActivate(sp.sprint.id)}>
                  <FiPlay size={14} />
                  Activate
                </button>
              )}
            </div>
            <p className="sprint-goal">{sp.sprint.goal}</p>
            <div className="sprint-dates">
              <FiCalendar size={14} />
              <span>{formatDate(sp.sprint.start_date)} - {formatDate(sp.sprint.end_date)}</span>
            </div>
            <div className="sprint-progress">
              <div className="progress-header">
                <span>{sp.completed_issues} of {sp.total_issues} issues</span>
                <span className="progress-percent">{Math.round(sp.velocity)}%</span>
              </div>
              <div className="progress-bar">
                <div
                  className="progress-fill"
                  style={{ width: `${sp.velocity}%` }}
                />
              </div>
            </div>
            <div className="sprint-stats-row">
              <div className="sprint-stat">
                <span className="sprint-stat-value">{sp.total_issues}</span>
                <span className="sprint-stat-label">Total</span>
              </div>
              <div className="sprint-stat">
                <span className="sprint-stat-value">{sp.completed_issues}</span>
                <span className="sprint-stat-label">Completed</span>
              </div>
              <div className="sprint-stat">
                <span className="sprint-stat-value">{sp.total_issues - sp.completed_issues}</span>
                <span className="sprint-stat-label">Remaining</span>
              </div>
            </div>
          </div>
        ))}
        {sprints.length === 0 && (
          <div className="empty-state">
            <p>No sprints created yet.</p>
          </div>
        )}
      </div>

      {showCreateModal && (
        <div className="modal-overlay" onClick={() => setShowCreateModal(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Create Sprint</h2>
              <button className="modal-close" onClick={() => setShowCreateModal(false)}>X</button>
            </div>
            <form className="modal-body" onSubmit={handleCreate}>
              <div className="form-group">
                <label>Sprint Name *</label>
                <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="Sprint 3" autoFocus required />
              </div>
              <div className="form-group">
                <label>Goal</label>
                <textarea value={goal} onChange={e => setGoal(e.target.value)} placeholder="What will this sprint achieve?" rows={3} />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>Start Date</label>
                  <input type="date" value={startDate} onChange={e => setStartDate(e.target.value)} />
                </div>
                <div className="form-group">
                  <label>End Date</label>
                  <input type="date" value={endDate} onChange={e => setEndDate(e.target.value)} />
                </div>
              </div>
              <div className="modal-footer">
                <button type="button" className="btn-secondary" onClick={() => setShowCreateModal(false)}>Cancel</button>
                <button type="submit" className="btn-primary" disabled={!name.trim() || isCreating}>
                  {isCreating ? 'Creating...' : 'Create Sprint'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
