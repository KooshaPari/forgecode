import { useState } from "react";
import {
  Issue,
  Priority,
  Status,
  PRIORITY_OPTIONS,
  STATUS_OPTIONS,
} from "../types";
import { useTauri } from "../hooks/useTauri";

interface IssueDetailProps {
  issue: Issue;
  onUpdated: () => void;
  onDeleted: () => void;
  onClose: () => void;
}

export default function IssueDetail({
  issue,
  onUpdated,
  onDeleted,
  onClose,
}: IssueDetailProps) {
  const [title, setTitle] = useState(issue.title);
  const [description, setDescription] = useState(issue.description);
  const [status, setStatus] = useState<Status>(issue.status as Status);
  const [priority, setPriority] = useState<Priority>(issue.priority as Priority);
  const [assignee, setAssignee] = useState(issue.assignee);
  const [labels, setLabels] = useState(issue.labels);
  const [saving, setSaving] = useState(false);
  const tauri = useTauri();

  const handleSave = async () => {
    setSaving(true);
    try {
      await tauri.updateIssue({
        id: issue.id,
        title,
        description,
        status,
        priority,
        assignee,
        labels,
      });
      onUpdated();
    } catch (err) {
      console.error("Failed to update issue:", err);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (confirm("Are you sure you want to delete this issue?")) {
      await tauri.deleteIssue(issue.id);
      onDeleted();
    }
  };

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  return (
    <div className="modal-overlay" onClick={handleOverlayClick}>
      <div className="modal">
        <div className="modal-header">
          <h2>Issue Details</h2>
          <button className="modal-close" onClick={onClose}>
            x
          </button>
        </div>
        <div className="modal-body">
          <div className="form-group">
            <label className="form-label">Title</label>
            <input
              className="form-input"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>

          <div className="form-group">
            <label className="form-label">Description</label>
            <textarea
              className="form-textarea"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Write a description (supports markdown)..."
            />
          </div>

          <div className="form-row">
            <div className="form-group">
              <label className="form-label">Status</label>
              <select
                className="form-select"
                value={status}
                onChange={(e) => setStatus(e.target.value as Status)}
              >
                {STATUS_OPTIONS.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-group">
              <label className="form-label">Priority</label>
              <select
                className="form-select"
                value={priority}
                onChange={(e) => setPriority(e.target.value as Priority)}
              >
                {PRIORITY_OPTIONS.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label className="form-label">Assignee</label>
              <input
                className="form-input"
                value={assignee}
                onChange={(e) => setAssignee(e.target.value)}
                placeholder="Assignee name"
              />
            </div>
            <div className="form-group">
              <label className="form-label">Labels (comma-separated)</label>
              <input
                className="form-input"
                value={labels}
                onChange={(e) => setLabels(e.target.value)}
                placeholder="bug, frontend, etc."
              />
            </div>
          </div>

          <div style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "8px" }}>
            Created: {new Date(issue.created_at).toLocaleString()}
            {" | "}
            Updated: {new Date(issue.updated_at).toLocaleString()}
          </div>
        </div>

        <div className="modal-footer">
          <button className="btn btn-danger" onClick={handleDelete}>
            Delete
          </button>
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? "Saving..." : "Save Changes"}
          </button>
        </div>
      </div>
    </div>
  );
}
