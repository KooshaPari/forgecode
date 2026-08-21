import React, { useState } from 'react';
import { FiX } from 'react-icons/fi';
import { api } from '../store';
import type { IssueStatus, IssuePriority } from '../types';

interface IssueCreateProps {
  onClose: () => void;
  onCreated: () => void;
}

export default function IssueCreate({ onClose, onCreated }: IssueCreateProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState<IssuePriority>('medium');
  const [status, setStatus] = useState<IssueStatus>('backlog');
  const [assignee, setAssignee] = useState('');
  const [labelsInput, setLabelsInput] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setIsSubmitting(true);
    try {
      await api.createIssue({
        title: title.trim(),
        description: description.trim(),
        priority,
        status,
        assignee: assignee.trim(),
        labels: labelsInput.split(',').map(l => l.trim()).filter(Boolean),
        sprint_id: null,
      });
      onCreated();
    } catch (err) {
      console.error('Failed to create issue:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Create Issue</h2>
          <button className="modal-close" onClick={onClose}><FiX size={20} /></button>
        </div>
        <form className="modal-body" onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="title">Title *</label>
            <input
              id="title"
              type="text"
              value={title}
              onChange={e => setTitle(e.target.value)}
              placeholder="Issue title"
              autoFocus
              required
            />
          </div>
          <div className="form-group">
            <label htmlFor="description">Description</label>
            <textarea
              id="description"
              value={description}
              onChange={e => setDescription(e.target.value)}
              placeholder="Describe the issue... (Markdown supported)"
              rows={5}
            />
          </div>
          <div className="form-row">
            <div className="form-group">
              <label htmlFor="priority">Priority</label>
              <select id="priority" value={priority} onChange={e => setPriority(e.target.value as IssuePriority)}>
                <option value="critical">Critical</option>
                <option value="high">High</option>
                <option value="medium">Medium</option>
                <option value="low">Low</option>
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="status">Status</label>
              <select id="status" value={status} onChange={e => setStatus(e.target.value as IssueStatus)}>
                <option value="backlog">Backlog</option>
                <option value="in-progress">In Progress</option>
                <option value="done">Done</option>
              </select>
            </div>
          </div>
          <div className="form-group">
            <label htmlFor="assignee">Assignee</label>
            <input
              id="assignee"
              type="text"
              value={assignee}
              onChange={e => setAssignee(e.target.value)}
              placeholder="Assignee name"
            />
          </div>
          <div className="form-group">
            <label htmlFor="labels">Labels (comma separated)</label>
            <input
              id="labels"
              type="text"
              value={labelsInput}
              onChange={e => setLabelsInput(e.target.value)}
              placeholder="bug, frontend, urgent"
            />
          </div>
          <div className="modal-footer">
            <button type="button" className="btn-secondary" onClick={onClose}>Cancel</button>
            <button type="submit" className="btn-primary" disabled={!title.trim() || isSubmitting}>
              {isSubmitting ? 'Creating...' : 'Create Issue'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
