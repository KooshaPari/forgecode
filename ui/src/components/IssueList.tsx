import { useState } from "react";
import {
  Issue,
  Priority,
  Status,
  PRIORITY_OPTIONS,
  STATUS_OPTIONS,
} from "../types";
import { useTauri } from "../hooks/useTauri";

interface IssueListProps {
  issues: Issue[];
  onSelectIssue: (issue: Issue) => void;
  onRefresh: () => void;
}

export default function IssueList({ issues, onSelectIssue, onRefresh }: IssueListProps) {
  const [filterPriority, setFilterPriority] = useState<Priority | "">("");
  const [filterStatus, setFilterStatus] = useState<Status | "">("");
  const [filterAssignee, setFilterAssignee] = useState("");
  const [searchTerm, setSearchTerm] = useState("");
  const tauri = useTauri();

  const filtered = issues.filter((issue) => {
    if (filterPriority && issue.priority !== filterPriority) return false;
    if (filterStatus && issue.status !== filterStatus) return false;
    if (
      filterAssignee &&
      !issue.assignee.toLowerCase().includes(filterAssignee.toLowerCase())
    )
      return false;
    if (
      searchTerm &&
      !issue.title.toLowerCase().includes(searchTerm.toLowerCase())
    )
      return false;
    return true;
  });

  const handleDelete = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    if (confirm("Delete this issue?")) {
      await tauri.deleteIssue(id);
      onRefresh();
    }
  };

  const formatDate = (d: string) => {
    return new Date(d).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  };

  return (
    <div>
      <div className="issue-list-toolbar">
        <input
          className="form-input"
          placeholder="Search issues..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          style={{ maxWidth: "240px" }}
        />
        <select
          className="form-select"
          value={filterStatus}
          onChange={(e) => setFilterStatus(e.target.value as Status | "")}
        >
          <option value="">All Statuses</option>
          {STATUS_OPTIONS.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <select
          className="form-select"
          value={filterPriority}
          onChange={(e) => setFilterPriority(e.target.value as Priority | "")}
        >
          <option value="">All Priorities</option>
          {PRIORITY_OPTIONS.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
        <input
          className="form-input"
          placeholder="Filter by assignee"
          value={filterAssignee}
          onChange={(e) => setFilterAssignee(e.target.value)}
          style={{ maxWidth: "180px" }}
        />
      </div>

      {filtered.length === 0 ? (
        <div className="empty-state">
          <div className="empty-state-icon">0</div>
          <h3>No issues found</h3>
          <p>Create an issue or adjust your filters.</p>
        </div>
      ) : (
        <table className="issue-table">
          <thead>
            <tr>
              <th>Title</th>
              <th>Status</th>
              <th>Priority</th>
              <th>Assignee</th>
              <th>Labels</th>
              <th>Created</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((issue) => (
              <tr key={issue.id} onClick={() => onSelectIssue(issue)}>
                <td style={{ fontWeight: 500 }}>{issue.title}</td>
                <td>
                  <span className={`status-badge ${issue.status.replace(" ", "-")}`}>
                    {issue.status}
                  </span>
                </td>
                <td>
                  <span className={`priority-badge ${issue.priority}`}>
                    {issue.priority}
                  </span>
                </td>
                <td style={{ color: "var(--text-secondary)" }}>
                  {issue.assignee || "\u2014"}
                </td>
                <td style={{ color: "var(--text-secondary)" }}>
                  {issue.labels || "\u2014"}
                </td>
                <td style={{ color: "var(--text-muted)", fontSize: "13px" }}>
                  {formatDate(issue.created_at)}
                </td>
                <td>
                  <button
                    className="btn btn-danger btn-sm"
                    onClick={(e) => handleDelete(e, issue.id)}
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
