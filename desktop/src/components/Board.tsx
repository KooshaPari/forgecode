import React, { useState, useRef, useCallback } from 'react';
import { FiMoreHorizontal } from 'react-icons/fi';
import { api } from '../store';
import type { Issue, IssueStatus } from '../types';

interface BoardProps {
  issues: Issue[];
  onIssueClick: (issue: Issue) => void;
  onIssueMoved: () => void;
}

const columns: { id: IssueStatus; title: string; color: string }[] = [
  { id: 'backlog', title: 'Backlog', color: 'var(--color-backlog)' },
  { id: 'in-progress', title: 'In Progress', color: 'var(--color-in-progress)' },
  { id: 'done', title: 'Done', color: 'var(--color-done)' },
];

export default function Board({ issues, onIssueClick, onIssueMoved }: BoardProps) {
  const [draggedIssue, setDraggedIssue] = useState<Issue | null>(null);
  const [dragOverColumn, setDragOverColumn] = useState<IssueStatus | null>(null);
  const dragRef = useRef<HTMLDivElement | null>(null);

  const handleDragStart = useCallback((e: React.DragEvent, issue: Issue) => {
    setDraggedIssue(issue);
    e.dataTransfer.effectAllowed = 'move';
    if (dragRef.current) {
      e.dataTransfer.setDragImage(dragRef.current, 0, 0);
    }
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, columnId: IssueStatus) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDragOverColumn(columnId);
  }, []);

  const handleDragLeave = useCallback(() => {
    setDragOverColumn(null);
  }, []);

  const handleDrop = useCallback(async (e: React.DragEvent, columnId: IssueStatus) => {
    e.preventDefault();
    setDragOverColumn(null);
    if (draggedIssue && draggedIssue.status !== columnId) {
      try {
        await api.updateIssue({ id: draggedIssue.id, status: columnId });
        onIssueMoved();
      } catch (err) {
        console.error('Failed to move issue:', err);
      }
    }
    setDraggedIssue(null);
  }, [draggedIssue, onIssueMoved]);

  const handleDragEnd = useCallback(() => {
    setDraggedIssue(null);
    setDragOverColumn(null);
  }, []);

  const getPriorityClass = (priority: string) => `priority-badge priority-${priority}`;

  return (
    <div className="board">
      {columns.map(column => {
        const columnIssues = issues.filter(i => i.status === column.id);
        const isDragOver = dragOverColumn === column.id;
        return (
          <div
            key={column.id}
            className={`board-column ${isDragOver ? 'drag-over' : ''}`}
            onDragOver={e => handleDragOver(e, column.id)}
            onDragLeave={handleDragLeave}
            onDrop={e => handleDrop(e, column.id)}
          >
            <div className="column-header">
              <div className="column-title-group">
                <span className="column-indicator" style={{ backgroundColor: column.color }} />
                <h3 className="column-title">{column.title}</h3>
                <span className="column-count">{columnIssues.length}</span>
              </div>
              <button className="column-menu-btn"><FiMoreHorizontal size={16} /></button>
            </div>
            <div className="column-body">
              {columnIssues.map(issue => (
                <div
                  key={issue.id}
                  className={`issue-card ${draggedIssue?.id === issue.id ? 'dragging' : ''}`}
                  draggable
                  onDragStart={e => handleDragStart(e, issue)}
                  onDragEnd={handleDragEnd}
                  onClick={() => onIssueClick(issue)}
                >
                  <div className="issue-card-header">
                    <span className={getPriorityClass(issue.priority)}>
                      {issue.priority}
                    </span>
                  </div>
                  <h4 className="issue-card-title">{issue.title}</h4>
                  <p className="issue-card-description">
                    {issue.description.length > 80
                      ? issue.description.substring(0, 80) + '...'
                      : issue.description || 'No description'}
                  </p>
                  <div className="issue-card-footer">
                    <div className="issue-card-labels">
                      {issue.labels.slice(0, 2).map((label, idx) => (
                        <span key={idx} className="issue-label">{label}</span>
                      ))}
                    </div>
                    <div className="issue-card-meta">
                      {issue.assignee ? (
                        <div className="issue-assignee" title={issue.assignee}>
                          {issue.assignee.split(' ').map(n => n[0]).join('').substring(0, 2).toUpperCase()}
                        </div>
                      ) : (
                        <div className="issue-assignee unassigned">?</div>
                      )}
                    </div>
                  </div>
                </div>
              ))}
              {columnIssues.length === 0 && (
                <div className="column-empty">
                  <span>No issues</span>
                </div>
              )}
            </div>
          </div>
        );
      })}
      <div className="board-drag-preview" ref={dragRef} />
    </div>
  );
}
