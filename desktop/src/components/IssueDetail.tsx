import React, { useState } from 'react';
import { FiX, FiEdit2, FiTrash2, FiArrowLeft } from 'react-icons/fi';
import { api } from '../store';
import type { Issue, IssueStatus, IssuePriority } from '../types';

interface IssueDetailProps {
  issue: Issue;
  onClose: () => void;
  onUpdated: () => void;
  onDeleted: () => void;
}

export default function IssueDetail({ issue, onClose, onUpdated, onDeleted }: IssueDetailProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [title, setTitle] = useState(issue.title);
  const [description, setDescription] = useState(issue.description);
  const [priority, setPriority] = useState<IssuePriority>(issue.priority);
  const [status, setStatus] = useState<IssueStatus>(issue.status);
  const [assignee, setAssignee] = useState(issue.assignee);
  const [labelsInput, setLabelsInput] = useState(issue.labels.join(', '));
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await api.updateIssue({
        id: issue.id,
        title,
        description,
        priority,
        status,
        assignee,
        labels: labelsInput.split(',').map(l => l.trim()).filter(Boolean),
      });
      onUpdated();
    } catch (err) {
      console.error('Failed to update issue:', err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm('Are you sure you want to delete this issue?')) return;
    try {
      await api.deleteIssue(issue.id);
      onDeleted();
    } catch (err) {
      console.error('Failed to delete issue:', err);
    }
  };

  const handleStatusChange = async (newStatus: IssueStatus) => {
    try {
      await api.updateIssue({ id: issue.id, status: newStatus });
      onUpdated();
    } catch (err) {
      console.error('Failed to update status:', err);
    }
  };

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleDateString('en-US', {
        year: 'numeric', month: 'short', day: 'numeric',
        hour: '2-digit', minute: '2-digit',
      });
    } catch {
      return dateStr;
    }
  };

  const renderMarkdown = (text: string) => {
    if (!text) return <span className="text-muted">No description provided.</span>;
    return text.split('\n').map((line, i) => {
      if (line.startsWith('# ')) return <h1 key={i}>{line.slice(2)}</h1>;
      if (line.startsWith('## ')) return <h2 key={i}>{line.slice(3)}</h2>;
      if (line.startsWith('### ')) return <h3 key={i}>{line.slice(4)}</h3>;
      if (line.startsWith('- ')) return <li key={i}>{line.slice(2)}</li>;
      if (line.startsWith('```')) return <code key={i} className="code-block">{line}</code>;
      if (line.startsWith('**') && line.endsWith('**')) return <strong key={i}>{line.slice(2, -2)}</strong>;
      if (line.startsWith('*') && line.endsWith('*')) return <em key={i}>{line.slice(1, -1)}</em>;
      if (line.trim() === '') return <br key={i} />;
      return <p key={i}>{line}</p>;
    });
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-large" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-header-left">
            <button className="btn-icon" onClick={onClose}><FiArrowLeft size={18} /></button>
            <h2>{isEditing ? 'Edit Issue' : issue.title}</h2>
          </div>
          <div className="modal-header-right">
            {!isEditing && (
              <>
                <button className="btn-icon" onClick={() => setIsEditing(true)}><FiEdit2 size={18} /></button>
                <button className="btn-icon btn-danger" onClick={handleDelete}><FiTrash2 size={18} /></button>
              </>
            )}
            <button className="modal-close" onClick={onClose}><FiX size={20} /></button>
          </div>
        </div>
        <div className="modal-body issue-detail">
          {isEditing ? (
            <div className="edit-form">
              <div className="form-group">
                <label>Title</label>
                <input type="text" value={title} onChange={e => setTitle(e.target.value)} />
              </div>
              <div className="form-group">
                <label>Description</label>
                <textarea value={description} onChange={e => setDescription(e.target.value)} rows={8} />
              </div>
              <div className="form-row">
                <div className="form-group">
                  <label>Priority</label>
                  <select value={priority} onChange={e => setPriority(e.target.value as IssuePriority)}>
                    <option value="critical">Critical</option>
                    <option value="high">High</option>
                    <option value="medium">Medium</option>
                    <option value="low">Low</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>Assignee</label>
                  <input type="text" value={assignee} onChange={e => setAssignee(e.target.value)} />
                </div>
              </div>
              <div className="form-group">
                <label>Labels (comma separated)</label>
                <input type="text" value={labelsInput} onChange={e => setLabelsInput(e.target.value)} />
              </div>
              <div className="edit-actions">
                <button className="btn-secondary" onClick={() => setIsEditing(false)}>Cancel</button>
                <button className="btn-primary" onClick={handleSave} disabled={isSaving}>
                  {isSaving ? 'Saving...' : 'Save Changes'}
                </button>
              </div>
            </div>
          ) : (
            <>
              <div className="detail-section">
                <div className="detail-meta">
                  <span className={`priority-badge priority-${issue.priority}`}>{issue.priority}</span>
                  <span className="status-badge">{issue.status}</span>
                  {issue.labels.map((label, i) => (
                    <span key={i} className="issue-label">{label}</span>
                  ))}
                </div>
              </div>
              <div className="detail-section">
                <h3>Status Actions</h3>
                <div className="status-actions">
                  <button
                    className={`status-btn ${status === 'backlog' ? 'active' : ''}`}
                    onClick={() => handleStatusChange('backlog')}
                  >
                    Backlog
                  </button>
                  <button
                    className={`status-btn ${status === 'in-progress' ? 'active' : ''}`}
                    onClick={() => handleStatusChange('in-progress')}
                  >
                    In Progress
                  </button>
                  <button
                    className={`status-btn ${status === 'done' ? 'active' : ''}`}
                    onClick={() => handleStatusChange('done')}
                  >
                    Done
                  </button>
                </div>
              </div>
              <div className="detail-section">
                <h3>Description</h3>
                <div className="markdown-content">
                  {renderMarkdown(issue.description)}
                </div>
              </div>
              <div className="detail-section detail-info">
                <div className="info-row">
                  <span className="info-label">Assignee</span>
                  <span className="info-value">{issue.assignee || 'Unassigned'}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">Created</span>
                  <span className="info-value">{formatDate(issue.created_at)}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">Updated</span>
                  <span className="info-value">{formatDate(issue.updated_at)}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">ID</span>
                  <span className="info-value info-id">{issue.id}</span>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
